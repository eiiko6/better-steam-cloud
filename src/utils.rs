use owo_colors::OwoColorize;

pub fn vprintln(verbose: bool, message: String) {
    if verbose {
        println!("{}", message.dimmed().bright_black());
    }
}
