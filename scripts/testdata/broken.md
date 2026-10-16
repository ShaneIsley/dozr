# Broken commands

Every `dozr` command in this file is wrong in a way that `scripts/check_examples.sh`
must report. CI runs the script on this file and expects it to fail. The
count of failures it must find is in the last line.

Inline, in prose, with a missing unit: `dozr d 5x -j 1s`.

A bare mention with one argument, such as `dozr d`, is not a command and is
not checked.

```sh
dozr n 1s 0.1
```

```bash
$ dozr par 0s 2
dozr d 5x -p 0.2 --skip-exit-code 2
dozr d 5s -p 0.2 --skip-exit-code 2 --bogus
dozr u 5s 1s -p 0.2 --skip-exit-code 1
dozr a 1ns
```

Expected failures: 7.
