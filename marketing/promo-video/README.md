# Automic Vault Product Promo Video

Remotion source for the Automic Vault product promo video.

The composition is `AutomicVaultPromo` at 1920x1080 and 30 fps. The rendered
output is ignored under `out/`.

## Commands

```sh
npm i
npm run dev
npm run lint
npm run render
```

Useful still checks:

```sh
npm run still -- out/stills/frame-14s.jpg --frame=421 --scale=0.25
npm run still -- out/stills/frame-24s.jpg --frame=720 --scale=0.25
npm run still -- out/stills/frame-33s.jpg --frame=990 --scale=0.25
```
