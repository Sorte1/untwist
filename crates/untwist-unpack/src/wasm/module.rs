use rustc_hash::FxHashMap;
use wasmparser::{ExternalKind, KnownCustom, Name, NameSectionReader, Parser, Payload, TypeRef};

use super::payload::{check_header, PayloadRefusal};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WasmImport {
    pub module: String,
    pub name: String,
    pub function_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WasmExport {
    pub name: String,
    pub function_name: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WasmSummary {
    pub imports: Vec<WasmImport>,
    pub exports: Vec<WasmExport>,
}

impl WasmSummary {
    pub fn import_modules(&self) -> Vec<&str> {
        let mut modules: Vec<&str> = self.imports.iter().map(|i| i.module.as_str()).collect();
        modules.sort_unstable();
        modules.dedup();
        modules
    }

    pub fn import(&self, module: &str, name: &str) -> Option<&WasmImport> {
        self.imports
            .iter()
            .find(|import| import.module == module && import.name == name)
    }

    pub fn export(&self, name: &str) -> Option<&WasmExport> {
        self.exports.iter().find(|export| export.name == name)
    }
}

#[derive(Default)]
struct Reading {
    imports: Vec<(WasmImport, Option<u32>)>,
    exports: Vec<(String, Option<u32>)>,
    names: FxHashMap<u32, String>,
    imported_functions: u32,
}

fn malformed(error: wasmparser::BinaryReaderError) -> PayloadRefusal {
    PayloadRefusal::Malformed(error.message().to_string())
}

impl Reading {
    fn imports(
        &mut self,
        reader: wasmparser::ImportSectionReader<'_>,
    ) -> Result<(), PayloadRefusal> {
        for import in reader.into_imports() {
            let import = import.map_err(malformed)?;
            let function = match import.ty {
                TypeRef::Func(_) | TypeRef::FuncExact(_) => {
                    self.imported_functions += 1;
                    Some(self.imported_functions - 1)
                }
                _ => None,
            };
            let entry = WasmImport {
                module: import.module.to_string(),
                name: import.name.to_string(),
                function_name: None,
            };
            self.imports.push((entry, function));
        }
        Ok(())
    }

    fn exports(
        &mut self,
        reader: wasmparser::ExportSectionReader<'_>,
    ) -> Result<(), PayloadRefusal> {
        for export in reader {
            let export = export.map_err(malformed)?;
            let function = matches!(export.kind, ExternalKind::Func).then_some(export.index);
            self.exports.push((export.name.to_string(), function));
        }
        Ok(())
    }

    fn names(&mut self, reader: NameSectionReader<'_>) {
        for name in reader.into_iter().flatten() {
            let Name::Function(map) = name else { continue };
            for naming in map.into_iter().flatten() {
                self.names.insert(naming.index, naming.name.to_string());
            }
        }
    }

    fn finish(mut self) -> WasmSummary {
        let names = std::mem::take(&mut self.names);
        let name_of = |index: Option<u32>| index.and_then(|index| names.get(&index).cloned());
        let imports = self
            .imports
            .into_iter()
            .map(|(import, index)| WasmImport {
                function_name: name_of(index),
                ..import
            })
            .collect();
        let exports = self
            .exports
            .into_iter()
            .map(|(name, index)| WasmExport {
                name,
                function_name: name_of(index),
            })
            .collect();
        WasmSummary { imports, exports }
    }
}

pub fn summarize(bytes: &[u8]) -> Result<WasmSummary, PayloadRefusal> {
    check_header(bytes)?;
    let mut reading = Reading::default();
    for payload in Parser::new(0).parse_all(bytes) {
        match payload.map_err(malformed)? {
            Payload::ImportSection(reader) => reading.imports(reader)?,
            Payload::ExportSection(reader) => reading.exports(reader)?,
            Payload::CustomSection(custom) => {
                if let KnownCustom::Name(reader) = custom.as_known() {
                    reading.names(reader);
                }
            }
            _ => {}
        }
    }
    Ok(reading.finish())
}
