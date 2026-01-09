use anyhow::Result;

pub fn handle_init(shell: &str) -> Result<()> {
    let script = match shell {
        "zsh" | "bash" => include_str!("../../scripts/init.sh"),
        "powershell" | "pwsh" => include_str!("../../scripts/init.ps1"),
        _ => "echo 'Unsupported shell'",
    };
    println!("{}", script);
    Ok(())
}
