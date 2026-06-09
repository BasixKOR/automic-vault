# Automic Vault Product Promo Video

Remotion source for the Automic Vault product promo videos.

The compositions are 1920x1080 at 30 fps. Rendered output is ignored under
`out/`.

- `AutomicVaultPromo`
- `AutomicVaultSkillSecrets`
- `AutomicVaultBrewInstallSecurity`
- `AutomicVaultScannerOneLiner`
- `AutomicVaultDontGetOwned`

## Commands

```sh
npm i
npm run dev
npm run lint
npm run render
npm run render:brew-install-security
npm run render:scanner-one-liner
npm run render:dont-get-owned
```

Useful still checks:

```sh
npm run still -- out/stills/frame-14s.jpg --frame=421 --scale=0.25
npm run still -- out/stills/frame-24s.jpg --frame=720 --scale=0.25
npm run still -- out/stills/frame-33s.jpg --frame=990 --scale=0.25
npm run still:brew-install-security -- out/stills/brew-act-1.jpg --frame=70 --scale=0.25
npm run still:brew-install-security -- out/stills/brew-act-2.jpg --frame=620 --scale=0.25
npm run still:brew-install-security -- out/stills/brew-act-3.jpg --frame=790 --scale=0.25
npm run still:scanner-one-liner -- out/stills/scanner-command.jpg --frame=96 --scale=0.25
npm run still:scanner-one-liner -- out/stills/scanner-findings.jpg --frame=420 --scale=0.25
npm run still:scanner-one-liner -- out/stills/scanner-end-card.jpg --frame=600 --scale=0.25
npm run still:dont-get-owned -- out/stills/dont-get-owned-detect.jpg --frame=172 --scale=0.25
npm run still:dont-get-owned -- out/stills/dont-get-owned-harden.jpg --frame=392 --scale=0.25
npm run still:dont-get-owned -- out/stills/dont-get-owned-owned.jpg --frame=610 --scale=0.25
```
