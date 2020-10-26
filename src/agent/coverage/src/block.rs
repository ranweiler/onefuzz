// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#[cfg(target_os = "windows")]
pub mod legacy;

#[cfg(target_os = "windows")]
pub mod windows;

use std::collections::BTreeSet;
use std::fmt;

use anyhow::Result;
use iced_x86::{Decoder, DecoderOptions, FlowControl, Instruction};
use symbolic::debuginfo::{Function, Object, ObjectError};

#[derive(Clone, Debug)]
pub struct Block {
    pub offset: u64,
    pub size: u64,
}

pub fn find_module_blocks(object: &Object) -> Result<Vec<Block>> {
    let session = object.debug_session().compat()?;

    let mut blocks = vec![];

    for function in session.functions() {
        let function = function.compat()?;
        let fn_blocks = find_function_blocks(object.data(), &function);
        blocks.extend(fn_blocks);
    }

    Ok(blocks)
}

pub fn find_function_blocks(data: &[u8], function: &Function) -> Vec<Block> {
    let leaders = find_function_leaders(data, function);

    let lo = function.address as usize;
    let hi = lo + (function.size as usize);
    let fn_data = &data[lo..hi];
    let mut decoder = Decoder::new(64, fn_data, DecoderOptions::NONE);

    // Enable using `Decoder::ip()` to get module-relative offsets.
    decoder.set_ip(function.address);

    let mut blocks = vec![];

    let mut offset = 0;
    let mut size = 0;

    let mut inst = Instruction::default();
    while decoder.can_decode() {
        let current = decoder.ip() as u64;

        decoder.decode_out(&mut inst);

        let leader = leaders.contains(&current);

        // May not point to an actual instruction in the function span.
        let next = decoder.ip() as u64;

        // If the next instruction is a leader, or the end of the function, then
        // we have reached the end of the block.
        let end = !decoder.can_decode() || leaders.contains(&next);

        size += inst.len() as u64;

        if leader {
            offset = current;
        }

        // Note: for a 1-instruction block, `leader && end`.
        if end {
            blocks.push(Block { offset, size });

            // Reset WIP block.
            offset = 0;
            size = 0;
        }
    }

    blocks
}

/// Compute the basic block leaders of a function as function-relative offsets.
fn find_function_leaders(data: &[u8], function: &Function) -> BTreeSet<u64> {
    let lo = function.address as usize;
    let hi = lo + (function.size as usize);
    let fn_data = &data[lo..hi];
    let mut decoder = Decoder::new(64, fn_data, DecoderOptions::NONE);
    decoder.set_ip(function.address);

    // Contains leaders with function-relative offsets.
    let mut leaders = BTreeSet::new();

    // Function entry is a leader.
    leaders.insert(function.address);

    let mut inst = Instruction::default();
    while decoder.can_decode() {
        decoder.decode_out(&mut inst);

        if let Some(target) = branch_target(&inst) {
            // The branch target is a leader.
            leaders.insert(target);

            // The next instruction is a leader, if it exists.
            if decoder.can_decode() {
                // We decoded the current instruction, so the decoder offset is
                // set to the next instruction.
                let next = decoder.ip() as u64;
                leaders.insert(next);
            }
        }
    }

    leaders
}

fn branch_target(inst: &Instruction) -> Option<u64> {
    match inst.flow_control() {
        FlowControl::ConditionalBranch
        | FlowControl::UnconditionalBranch
        | FlowControl::IndirectBranch => Some(inst.near_branch_target()),
        _ => None,
    }
}

// Wrapper to enable us to `impl Error` for foreign type.
struct Compat(symbolic::debuginfo::ObjectError);

impl std::error::Error for Compat {}

impl fmt::Debug for Compat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for Compat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

trait ErrorCompat<T> {
    fn compat(self) -> std::result::Result<T, Compat>;
}

impl<T> ErrorCompat<T> for std::result::Result<T, ObjectError> {
    fn compat(self) -> std::result::Result<T, Compat> {
        self.map_err(Compat)
    }
}
