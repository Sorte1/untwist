use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

pub const OUT: &str = "fixtures/generated/layers";
const BUNDLE: &str = "fixtures/generated/webpack-5mb.js";
const SMALL: &str = "fixtures/reference/deobfuscate/obfuscator.io-high.js";
const PACK: &str = "tools/layers/pack.mjs";
const CHAIN_DEPTHS: [usize; 4] = [1, 2, 4, 8];
const PACKER_WORD_BYTES: usize = 4096;
const PACKER_REPEATS: usize = 4608;
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

#[derive(clap::Args, Debug)]
pub struct Request {
    #[arg(long)]
    pub out: Option<PathBuf>,
    #[arg(long)]
    pub skip_packed: bool,
}

pub fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, shift) in [18u32, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[((n >> shift) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

pub fn wrap_atob(payload: &str, depth: usize) -> String {
    let mut text = payload.to_string();
    for _ in 0..depth {
        text = format!("eval(atob(\"{}\"));\n", base64(text.as_bytes()));
    }
    text
}

pub fn oversized_packer() -> String {
    let word = "w".repeat(PACKER_WORD_BYTES);
    let body = "0;".repeat(PACKER_REPEATS);
    format!(
        "eval(function(p,a,c,k,e,r){{e=String;if(!''.replace(/^/,String)){{while(c--)r[c]=k[c]||c;\
         k=[function(e){{return r[e]}}];e=function(){{return'\\\\w+'}};c=1}};\
         while(c--)if(k[c])p=p.replace(new RegExp('\\\\b'+e(c)+'\\\\b','g'),k[c]);return p}}\
         ('2 0=1;{body}3.4(5 0)',10,6,'{word}||var|console|log|typeof'.split('|'),0,{{}}))\n"
    )
}

fn write(path: &Path, text: &str) -> Result<()> {
    std::fs::write(path, text).with_context(|| format!("write {}", path.display()))?;
    println!("wrote {} ({} bytes)", path.display(), text.len());
    Ok(())
}

fn pack_with_node(root: &Path, input: &Path, output: &Path) -> Result<()> {
    let status = Command::new("node")
        .arg(root.join(PACK))
        .arg(input)
        .arg(output)
        .status()
        .context("spawn node for tools/layers/pack.mjs")?;
    if !status.success() {
        bail!("tools/layers/pack.mjs exited {status}");
    }
    let bytes = std::fs::metadata(output)?.len();
    println!("wrote {} ({bytes} bytes)", output.display());
    Ok(())
}

pub fn run(root: &Path, request: &Request) -> Result<()> {
    let out = request.out.clone().unwrap_or_else(|| root.join(OUT));
    std::fs::create_dir_all(&out).with_context(|| format!("create {}", out.display()))?;
    let bundle_path = root.join(BUNDLE);
    let bundle = std::fs::read_to_string(&bundle_path).with_context(|| {
        format!(
            "read {} (run cargo xtask corpus-bundle first)",
            bundle_path.display()
        )
    })?;
    write(&out.join("atob4-5mb.js"), &wrap_atob(&bundle, 4))?;
    let small = std::fs::read_to_string(root.join(SMALL)).context("read the high preset")?;
    for depth in CHAIN_DEPTHS {
        write(
            &out.join(format!("atob{depth}-high.js")),
            &wrap_atob(&small, depth),
        )?;
    }
    write(&out.join("oversized-packer.js"), &oversized_packer())?;
    if !request.skip_packed {
        pack_with_node(root, &bundle_path, &out.join("packed-5mb.js"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{base64, oversized_packer, PACKER_REPEATS, PACKER_WORD_BYTES};

    #[test]
    fn base64_matches_the_reference_alphabet_and_padding() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn the_oversized_packer_payload_is_over_sixteen_mib_from_a_small_input() {
        let text = oversized_packer();
        let payload = PACKER_WORD_BYTES * (PACKER_REPEATS + 2);
        assert!(payload > 16 << 20);
        assert!(payload > 4 * text.len());
    }
}
