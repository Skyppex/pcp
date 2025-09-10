# progress

when starting the copy job for a given destination, create a new file which
holds a new line separated list of completed file names in the destination.
these should be relative to the destination directory to avoid it breaking if
the destination moves between work.

when a file starts it copying job, create another file with the same name under
.pcp/ but with the .pcp extension

after every chunk is written, update the progress file with the total number of
bytes written
