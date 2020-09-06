# ddh-remover
A simple program to remove duplicates files from ddh

It takes as input a json from the [ddh](https://github.com/darakian/ddh) utility and it removes the duplicate files.

## Usage
It's a CLI utility. It takes input either from a file or stdin.
One note, the json output from ddh adds some extra line at the beginning to show what is has found.
I normally parse it like this to have a clean json output:

```
# ddh <dir> -o no -v all -f json | awk '/\[/,/\]/' > output.json

```
Or:
```
# ddh <dir> -o no -v all -f json | awk '/\[/,/\]/' | ddh-remover

```

## CLI Example
```
ddh-remover 0.1
Pierguido L.
It removes files found by the ddh utility.
ddh has to be used with the json output to be parsed by ddh-remover.
This can be saved in a file or read from stdin with a pipe a pipe

USAGE:
    ddh-remover [FLAGS] [OPTIONS]

FLAGS:
    -h, --help
            Prints help information

    -n
            It doesn't do anything, no file removal

    -V, --version
            Prints version information


OPTIONS:
    -m, --move <dest_path>
            Move the files to [dest_path] instead of deleting them

    -d, --duplicates <duplicates>
            How many duplicates to keep. Defaults to 1 (only one file, no duplicates) [default: 1]

    -f, --file <file>
            Read the json input from a file
```

### NOTE: this may have los of bugs, I'm not responsible for lost important files.
