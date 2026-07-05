# nuget

NuGet stores package source credentials, API keys, proxy passwords, and client
certificate passwords in user-level `NuGet.Config` files.

This radioisotope migrates the default Mono and .NET user-level NuGet config
files to the keychain. Package source credentials are split out for
`av credential-helper nuget`, while API keys, proxy passwords, and client
certificate passwords remain in sanitized temporary config files because NuGet's
credential provider protocol does not cover those secret types.
