use super::*;

/// Open Railway Documentation in default browser
#[derive(Parser)]
pub struct Args {}

pub async fn command(args: Args) -> Result<()> {
    println!("🚝 Press Enter to open the browser (^C to quit)");
    let mut temp = String::new();
    std::io::stdin().read_line(&mut temp)?;
    match ::open::that("https://docs.railway.app/") {
        Ok(_) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
