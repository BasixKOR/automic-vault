# midnight-commander

Midnight Commander can store VFS credentials in its user profile, including
FTP settings in `ini` and remote locations with embedded passwords in
`hotlist`.

This radioisotope migrates the relevant MC profile files to the keychain and
wraps `mc` so those files are recreated under a temporary `MC_PROFILE_ROOT`
while Midnight Commander runs.

