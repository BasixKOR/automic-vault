# cariddi Radioisotope

Detect-only coverage for cariddi scan inputs and findings.

cariddi can receive authentication headers on the command line or in a headers
file, and its default output directory can contain discovered secrets. This
radioisotope reports those obvious local exposures without changing cariddi's
scan behavior.
