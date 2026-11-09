use rustc_hash::FxHashMap;
use wasmparser::{
    CompositeInnerType, ExternalKind, FuncType, KnownCustom, Name, NameSectionReader, Parser,
    Payload, TypeRef, ValType,
};

use super::payload::{check_header, PayloadRefusal};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueType {
    I32,
    I64,
    F32,
    F64,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature {
    pub params: Vec<ValueType>,
    pub results: Vec<ValueType>,
}

impl Signature {
    fn of(func: &FuncType) -> Self {
        let kinds = |types: &[ValType]| types.iter().map(|ty| value_type(*ty)).collect();
        Self {
            params: kinds(func.params()),
            results: kinds(func.results()),
        }
    }
}

fn value_type(ty: ValType) -> ValueType {
    match ty {
        ValType::I32 => ValueType::I32,
        ValType::I64 => ValueType::I64,
        ValType::F32 => ValueType::F32,
        ValType::F64 => ValueType::F64,
        _ => ValueType::Other,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WasmImport {
    pub module: String,
    pub name: String,
    pub function_name: Option<String>,
    pub signature: Option<Signature>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportKind {
    Function,
    Memory,
    Table,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WasmExport {
    pub name: String,
    pub kind: ExportKind,
    pub function_name: Option<String>,
    pub signature: Option<Signature>,
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

    pub fn has_function_names(&self) -> bool {
        self.imports
            .iter()
            .any(|import| import.function_name.is_some())
            || self
                .exports
                .iter()
                .any(|export| export.function_name.is_some())
    }

    pub fn is_inert(&self) -> bool {
        self.imports.is_empty() && self.exports.is_empty()
    }
}

#[derive(Default)]
struct Reading {
    imports: Vec<(WasmImport, Option<u32>)>,
    exports: Vec<(String, ExportKind, Option<u32>)>,
    names: FxHashMap<u32, String>,
    imported_functions: u32,
    types: Vec<Option<Signature>>,
    function_types: Vec<u32>,
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
            let (function, signature) = match import.ty {
                TypeRef::Func(ty) | TypeRef::FuncExact(ty) => {
                    self.imported_functions += 1;
                    (Some(self.imported_functions - 1), self.signature(ty))
                }
                _ => (None, None),
            };
            let entry = WasmImport {
                module: import.module.to_string(),
                name: import.name.to_string(),
                function_name: None,
                signature,
            };
            self.imports.push((entry, function));
        }
        Ok(())
    }

    fn signature(&self, ty: u32) -> Option<Signature> {
        self.types.get(ty as usize).cloned().flatten()
    }

    fn types(&mut self, reader: wasmparser::TypeSectionReader<'_>) -> Result<(), PayloadRefusal> {
        for group in reader {
            for ty in group.map_err(malformed)?.into_types() {
                self.types.push(match &ty.composite_type.inner {
                    CompositeInnerType::Func(func) => Some(Signature::of(func)),
                    _ => None,
                });
            }
        }
        Ok(())
    }

    fn functions(
        &mut self,
        reader: wasmparser::FunctionSectionReader<'_>,
    ) -> Result<(), PayloadRefusal> {
        for ty in reader {
            self.function_types.push(ty.map_err(malformed)?);
        }
        Ok(())
    }

    fn function_signature(&self, index: u32) -> Option<Signature> {
        let ty = match index.checked_sub(self.imported_functions) {
            Some(defined) => *self.function_types.get(defined as usize)?,
            None => {
                return self.imports.iter().find_map(|(import, function)| {
                    (*function == Some(index))
                        .then(|| import.signature.clone())
                        .flatten()
                })
            }
        };
        self.signature(ty)
    }

    fn exports(
        &mut self,
        reader: wasmparser::ExportSectionReader<'_>,
    ) -> Result<(), PayloadRefusal> {
        for export in reader {
            let export = export.map_err(malformed)?;
            let kind = match export.kind {
                ExternalKind::Func | ExternalKind::FuncExact => ExportKind::Function,
                ExternalKind::Memory => ExportKind::Memory,
                ExternalKind::Table => ExportKind::Table,
                _ => ExportKind::Other,
            };
            let function = (kind == ExportKind::Function).then_some(export.index);
            self.exports.push((export.name.to_string(), kind, function));
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
        let signatures: Vec<Option<Signature>> = self
            .exports
            .iter()
            .map(|(_, _, index)| index.and_then(|index| self.function_signature(index)))
            .collect();
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
            .zip(signatures)
            .map(|((name, kind, index), signature)| WasmExport {
                name,
                kind,
                function_name: name_of(index),
                signature,
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
            Payload::TypeSection(reader) => reading.types(reader)?,
            Payload::ImportSection(reader) => reading.imports(reader)?,
            Payload::FunctionSection(reader) => reading.functions(reader)?,
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
