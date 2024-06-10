# SysKill
A command line tool built using Rust, Ratatui, and Sysinfo for finding and terminating processes.

![image](https://github.com/alexei-ozerov/syskill/assets/44589006/2aa2363b-b8be-44a8-bc24-a71666f673d5)

## Installation
1. Clone this repository.
2. Run `cargo build --release`, and move the binary to the desired place `mv syskill/target/release/syskill <target directory present in path>`.
3. OPTIONAL: Add an alias to your bashrc to run the util directly from the target folder using something like `alias sk="~/<path to syskill directory>/syskill/target/release/syskill"`. Don't forget to run `source` against the file in which your alias lives.

## Usage
The `j` and `k` keys allow you to scroll up and down through the process table. The `d` key allows you to kill a highlighted processes. The `r` key refreshes the list of processes. The `q` key exits the application.
