A command line tool for turning images into instructions for building a minecraft map art.

USAGE:
    minecraft_image [FLAGS] [ARGS]

ARGS:
    <file>               
    <output-location>    [default: output.png]

FLAGS:
    -d, --dither       use dithering
    -h, --help         Prints help information
    -s, --staircase     whether to use staircase method or not, staircase method usually produces
                       much better results, but is significantly more complicated
    -u, --use-mask     whether to only use the blocks listed in "mask.txt"
    -V, --version      Prints version information
