cmake_minimum_required(VERSION 3.16)
project(opentag_bzip C)
set(BZIP2_SOURCE_DIR "" CACHE PATH "Verified bzip2 source directory")
add_library(bz2 STATIC
  "${BZIP2_SOURCE_DIR}/blocksort.c"
  "${BZIP2_SOURCE_DIR}/huffman.c"
  "${BZIP2_SOURCE_DIR}/crctable.c"
  "${BZIP2_SOURCE_DIR}/randtable.c"
  "${BZIP2_SOURCE_DIR}/compress.c"
  "${BZIP2_SOURCE_DIR}/decompress.c"
  "${BZIP2_SOURCE_DIR}/bzlib.c")
install(TARGETS bz2 ARCHIVE DESTINATION lib)
install(FILES "${BZIP2_SOURCE_DIR}/bzlib.h" DESTINATION include)
