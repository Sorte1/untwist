#![allow(dead_code)]

pub const I32: u8 = 0x7f;
pub const F64: u8 = 0x7c;

const BASE64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let word = chunk.iter().enumerate().fold(0u32, |word, (index, byte)| {
            word | u32::from(*byte) << (16 - 8 * index)
        });
        for index in 0..4 {
            match index <= chunk.len() {
                true => out.push(BASE64[(word >> (18 - 6 * index) & 63) as usize] as char),
                false => out.push('='),
            }
        }
    }
    out
}

#[derive(Clone, Copy)]
pub struct Sig<'a> {
    pub params: &'a [u8],
    pub results: &'a [u8],
}

pub const fn sig<'a>(params: &'a [u8], results: &'a [u8]) -> Sig<'a> {
    Sig { params, results }
}

#[derive(Default)]
pub struct Module<'a> {
    pub imports: Vec<(&'a str, &'a str, Sig<'a>)>,
    pub functions: Vec<(&'a str, Sig<'a>)>,
    pub memory: Option<&'a str>,
}

fn leb(mut value: usize, out: &mut Vec<u8>) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        match value {
            0 => return out.push(byte),
            _ => out.push(byte | 0x80),
        }
    }
}

fn name(text: &str, out: &mut Vec<u8>) {
    leb(text.len(), out);
    out.extend_from_slice(text.as_bytes());
}

fn section(id: u8, items: &[Vec<u8>], out: &mut Vec<u8>) {
    if items.is_empty() {
        return;
    }
    let mut body = Vec::new();
    leb(items.len(), &mut body);
    items.iter().for_each(|item| body.extend_from_slice(item));
    out.push(id);
    leb(body.len(), out);
    out.extend_from_slice(&body);
}

fn encoded_sig(sig: Sig<'_>) -> Vec<u8> {
    let mut out = vec![0x60];
    leb(sig.params.len(), &mut out);
    out.extend_from_slice(sig.params);
    leb(sig.results.len(), &mut out);
    out.extend_from_slice(sig.results);
    out
}

fn body_for(sig: Sig<'_>) -> Vec<u8> {
    let mut code = vec![0x00];
    for result in sig.results {
        match *result {
            F64 => code.extend_from_slice(&[0x44, 0, 0, 0, 0, 0, 0, 0, 0]),
            _ => code.extend_from_slice(&[0x41, 0x00]),
        }
    }
    code.push(0x0b);
    let mut out = Vec::new();
    leb(code.len(), &mut out);
    out.extend_from_slice(&code);
    out
}

impl Module<'_> {
    pub fn bytes(&self) -> Vec<u8> {
        let sigs: Vec<Sig<'_>> = self
            .imports
            .iter()
            .map(|(_, _, sig)| *sig)
            .chain(self.functions.iter().map(|(_, sig)| *sig))
            .collect();
        let types: Vec<Vec<u8>> = sigs.iter().map(|sig| encoded_sig(*sig)).collect();
        let imports: Vec<Vec<u8>> = self
            .imports
            .iter()
            .enumerate()
            .map(|(index, (module, field, _))| {
                let mut out = Vec::new();
                name(module, &mut out);
                name(field, &mut out);
                out.push(0x00);
                leb(index, &mut out);
                out
            })
            .collect();
        let first = self.imports.len();
        let functions: Vec<Vec<u8>> = (0..self.functions.len())
            .map(|index| {
                let mut out = Vec::new();
                leb(first + index, &mut out);
                out
            })
            .collect();
        let memories: Vec<Vec<u8>> = self.memory.map(|_| vec![0x00, 0x01]).into_iter().collect();
        let mut exports: Vec<Vec<u8>> = Vec::new();
        if let Some(memory) = self.memory {
            let mut out = Vec::new();
            name(memory, &mut out);
            out.extend_from_slice(&[0x02, 0x00]);
            exports.push(out);
        }
        for (index, (export, _)) in self.functions.iter().enumerate() {
            let mut out = Vec::new();
            name(export, &mut out);
            out.push(0x00);
            leb(first + index, &mut out);
            exports.push(out);
        }
        let codes: Vec<Vec<u8>> = self
            .functions
            .iter()
            .map(|(_, sig)| body_for(*sig))
            .collect();
        let mut out = b"\0asm\x01\0\0\0".to_vec();
        section(1, &types, &mut out);
        section(2, &imports, &mut out);
        section(3, &functions, &mut out);
        section(5, &memories, &mut out);
        section(7, &exports, &mut out);
        section(10, &codes, &mut out);
        out
    }
}
