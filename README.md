# chousei (調整)

Japanese for "adjustment"

Shift the timestamps of an SRT subtitle file

# Usage

First build with `cargo build --release`

Then if you'd like, link the binary into the bin directory to it's available in the path: `cargo install --path . --locked`

Simply pass in a list of file paths to be converted

`chousei input.srt -0:09`

Now input.srt will have the "00:00:01,000 --> 00:00:02,000" timestamps adjusted to be 9 seconds earlier

Use the `-h` or `--help` flag for more information
