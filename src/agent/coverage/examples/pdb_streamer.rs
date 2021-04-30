// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use std::fs;

use anyhow::Result;
use coverage::pdb::{PdbStreamer, Visit};
use pdb::{
    AddressMap, DataSymbol, DebugInformation, ProcedureSymbol, Symbol, TypeData, TypeIndex,
    TypeInformation, PDB,
};
use structopt::StructOpt;

#[derive(Debug, PartialEq, StructOpt)]
struct Opt {
    #[structopt(long)]
    pdb: std::path::PathBuf,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let pdb = fs::File::open(&opt.pdb)?;
    let mut pdb = PDB::open(pdb)?;

    let address_map = pdb.address_map()?;
    let type_information = pdb.type_information()?;
    let mut visitor = Visitor::new(address_map, type_information)?;

    let mut streamer = PdbStreamer::new(&mut pdb);

    streamer.stream_debug_info(&mut visitor)?;
    streamer.stream_global_symbols(&mut visitor)?;

    Ok(())
}

pub struct Visitor<'am, 'ti> {
    address_map: AddressMap<'am>,
    type_information: TypeInformation<'ti>,
}

impl<'am, 'ti> Visitor<'am, 'ti> {
    pub fn new(
        address_map: AddressMap<'am>,
        type_information: TypeInformation<'ti>,
    ) -> Result<Self> {
        Ok(Self {
            address_map,
            type_information,
        })
    }

    fn find_type(&self, index: TypeIndex) -> Result<Option<TypeData>> {
        use pdb::FallibleIterator;

        let ty = self
            .type_information
            .iter()
            .find(|ty| Ok(ty.index() == index))?;

        if let Some(ty) = ty {
            let parsed = ty.parse()?;
            return Ok(Some(parsed));
        }

        Ok(None)
    }
}

impl<'am, 'ti> coverage::pdb::Visitor for Visitor<'am, 'ti> {
    fn visit_debug_info(&mut self, debug_info: &DebugInformation) -> Result<Visit> {
        println!("{:#x?}", debug_info);

        Ok(Visit::Continue)
    }

    fn visit_data_symbol(&mut self, _symbol: Symbol, data: DataSymbol) -> Result<Visit> {
        let rva = data.offset.to_rva(&self.address_map).unwrap_or_default();

        let ty = self.find_type(data.type_index)?;
        let ty_name = type_name(&ty);

        println!("[data] {}, {}, {}", rva, ty_name, data.name);

        Ok(Visit::Continue)
    }

    fn visit_procedure_symbol(&mut self, _symbol: Symbol, proc: ProcedureSymbol) -> Result<Visit> {
        let rva = proc.offset.to_rva(&self.address_map).unwrap_or_default();

        let ty = self.find_type(proc.type_index)?;
        let ty_name = type_name(&ty);

        println!("[proc] {}, {}, {}", rva, ty_name, proc.name);

        Ok(Visit::Continue)
    }
}

fn type_name(ty: &Option<TypeData>) -> String {
    if let Some(parsed) = ty {
        parsed
            .name()
            .map(|raw| raw.to_string().into())
            .unwrap_or_else(|| "<type:unknown>".into())
    } else {
        "<type:none>".into()
    }
}
