# stdin

spec for how to handle input from stdin

## format

the input is a list of operations which all share the flags passed to the pcp
command itself. each operation is defined on a single line, always. 

each lines is a list of paths. the first path is the source and the following
paths are all destination paths where the source path will end up. each path is
separated by a colon `:` like this:

```bash
a/file0:b/file0
a/file1:b/file1
a/file2:b/file2
```

lines starting with a `#` will be ignored
```bash
# copy from a to b
a/file0:b/file0
a/file1:b/file1
a/file2:b/file2
```

multiple destinations:
```bash
a/file0:b/file0:c/file0
a/file1:b/file1:c/file1
a/file2:b/file2:c/file2
```

with this format you can rename during the copy:
```bash
# copy/move and rename
a/file0:b/file1
# distribute and rename
a/file1:b/good-file:c/bad-file
# create several copies in the same directory
a/file0:a/file1:a/file2:a/file3:a/file4:a/file5
```

## impl

each operation will be handled in a thread which is further split into threads
for parallel processing.
