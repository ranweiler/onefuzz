// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use anyhow::Result;
use pdb::{
    DataSymbol, DebugInformation, FallibleIterator, ProcedureSymbol, Symbol, SymbolData, PDB,
};

pub struct PdbStreamer<'pdb, 'd, D> {
    pdb: &'pdb mut PDB<'d, D>,
}

impl<'pdb, 'd, D> PdbStreamer<'pdb, 'd, D>
where
    D: pdb::Source<'d> + 'd,
{
    pub fn new(pdb: &'pdb mut PDB<'d, D>) -> Self {
        Self { pdb }
    }

    pub fn stream_debug_info(&mut self, visitor: &mut impl Visitor) -> Result<()> {
        let debug_info = self.pdb.debug_information()?;

        let next = visitor.visit_debug_info(&debug_info)?;
        if next.stop() {
            return Ok(());
        }

        // PDB "modules" correspond to object files or import libraries.
        let mut modules = debug_info.modules()?;

        while let Some(module) = modules.next()? {
            if let Some(module_info) = self.pdb.module_info(&module)? {
                let mut symbols = module_info.symbols()?;
                while let Some(symbol) = symbols.next()? {
                    let visit = self.stream_symbol(symbol, visitor)?;

                    if visit.stop() {
                        return Ok(());
                    }
                }
            }
        }

        Ok(())
    }

    pub fn stream_global_symbols(&mut self, visitor: &mut impl Visitor) -> Result<()> {
        let global_symbols = self.pdb.global_symbols()?;
        let mut symbols = global_symbols.iter();

        while let Some(symbol) = symbols.next()? {
            let visit = self.stream_symbol(symbol, visitor)?;

            if visit.stop() {
                return Ok(());
            }
        }

        Ok(())
    }

    fn stream_symbol(&mut self, symbol: Symbol, visitor: &mut impl Visitor) -> Result<Visit> {
        // Stream the unparsed symbol.
        let visit = visitor.visit_symbol(&symbol)?;

        if visit.stop() {
            return Ok(visit);
        }

        // Try to parse the symbol and stream both it and its data.
        //
        // For now, we only stream data and procedure symbols.
        if let Ok(parsed) = symbol.parse() {
            match parsed {
                SymbolData::Data(data) => {
                    return visitor.visit_data_symbol(symbol, data);
                }
                SymbolData::Procedure(proc) => {
                    return visitor.visit_procedure_symbol(symbol, proc);
                }
                _ => {}
            }
        }

        Ok(Visit::Continue)
    }
}

pub enum Visit {
    Continue,
    Stop,
}

impl Visit {
    pub fn stop(&self) -> bool {
        match self {
            Visit::Continue => false,
            Visit::Stop => true,
        }
    }
}

pub trait Visitor {
    fn visit_debug_info(&mut self, _debug_info: &DebugInformation) -> Result<Visit> {
        Ok(Visit::Continue)
    }

    fn visit_symbol(&mut self, _symbol: &Symbol) -> Result<Visit> {
        Ok(Visit::Continue)
    }

    fn visit_data_symbol(&mut self, _symbol: Symbol, _data: DataSymbol) -> Result<Visit> {
        Ok(Visit::Continue)
    }

    fn visit_procedure_symbol(&mut self, _symbol: Symbol, _proc: ProcedureSymbol) -> Result<Visit> {
        Ok(Visit::Continue)
    }
}
