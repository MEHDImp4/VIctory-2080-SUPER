//! Opt-in render-scale hook from trustno1-coder/VIctory-NVIDIA afd84de.
//! It changes the mapped image of the known 1.0.2.1 build before startup.

use anyhow::{Context, Result, ensure};
use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
use windows_sys::Win32::System::Memory::{PAGE_EXECUTE_READWRITE, VirtualProtect};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

const BUILD: &str = "b99f876b74d4a77f03af5f878223b27bd647b5c35a79d5decd9f716504239e93";
const SCALE: &str = "VICTORY_RENDER_SCALE";

fn parse_scale(text: &str) -> Option<f32> {
    let value = text.trim().parse::<f32>().ok()?;
    (0.2..1.0).contains(&value).then_some(value)
}

/// Force the title's temporal upscaler and fix its minimum and maximum scale.
///
/// # Safety
/// `base` is the live mapping of the executable whose original hash is `hash`.
/// No title code can execute until the caller has checked the result.
pub(crate) unsafe fn render_scale(base: usize, hash: &blake3::Hash) -> Result<()> {
    let Some(value) = std::env::var_os(SCALE) else {
        return Ok(());
    };
    let scale = parse_scale(&value.to_string_lossy())
        .context("VICTORY_RENDER_SCALE must be at least 0.2 and less than 1.0")?;
    ensure!(
        hash.to_hex().as_str() == BUILD,
        "the render scale hook needs the known title build 1.0.2.1"
    );
    let scale_bytes = scale.to_le_bytes();
    let patches: [(usize, &[u8], &[u8]); 3] = [
        (0x7e9426, &[0x0F, 0x45, 0xC1], &[0x0F, 0x1F, 0x00]),
        (0x28f1e7e, &[0xAB, 0xAA, 0x2A, 0x3F], &scale_bytes),
        (
            0x28f1e87,
            &[0xC5, 0xFA, 0x5F, 0x86, 0x90, 0, 0, 0],
            &[0xC5, 0xFA, 0x10, 0x86, 0x90, 0, 0, 0],
        ),
    ];
    for (rva, expected, _) in patches {
        // SAFETY: these RVAs are inside the executable verified above.
        let bytes =
            unsafe { std::slice::from_raw_parts((base + rva) as *const u8, expected.len()) };
        ensure!(
            bytes == expected,
            "the mapped render scale instruction at {rva:#x} has unexpected bytes"
        );
    }
    for (rva, _, replacement) in patches {
        let location = (base + rva) as *mut u8;
        let mut old = 0;
        // SAFETY: all targets were verified and none can execute while patched.
        unsafe {
            if VirtualProtect(
                location.cast(),
                replacement.len(),
                PAGE_EXECUTE_READWRITE,
                &mut old,
            ) == 0
            {
                return Err(std::io::Error::last_os_error())
                    .context("making the render scale instruction writable");
            }
            std::ptr::copy_nonoverlapping(replacement.as_ptr(), location, replacement.len());
            let mut previous = 0;
            if VirtualProtect(location.cast(), replacement.len(), old, &mut previous) == 0 {
                return Err(std::io::Error::last_os_error())
                    .context("restoring the render scale instruction protection");
            }
            if FlushInstructionCache(GetCurrentProcess(), location.cast(), replacement.len()) == 0 {
                return Err(std::io::Error::last_os_error())
                    .context("flushing the render scale instruction cache");
            }
        }
    }
    tracing::info!(
        scale,
        "using the title's temporal upscaler at a fixed render scale"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_scale;

    #[test]
    fn render_scale_must_be_finite_and_below_native() {
        assert_eq!(parse_scale("0.5"), Some(0.5));
        assert_eq!(parse_scale(" 0.2 "), Some(0.2));
        for invalid in ["0.1", "1", "2", "NaN", "inf", "half"] {
            assert_eq!(parse_scale(invalid), None);
        }
    }
}
