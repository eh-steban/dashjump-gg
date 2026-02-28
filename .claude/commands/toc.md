Show the section structure of a file with line numbers, then read only the
section you need. Useful for large files where you don't want to load
everything into context.

Usage: /toc [filepath]

Steps:
1. Run: grep -n "^## \|^### \|^# " $ARGUMENTS | head -40
2. Review the headers and line numbers
3. Use Read with a line range to load only the relevant section
   Example: Read private/specs/NNN-feature.md lines 45-78

This avoids loading an entire spec or plan file when you only need one section.
