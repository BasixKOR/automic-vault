# Automic Vault Promo Video

Remotion source for the Automic Vault promo videos.

The compositions are `AutomicVaultPromo` and `AutomicVaultFounderStory` at
1920x1080 and 30 fps. The rendered output is ignored under `out/`.

## Commands

```sh
npm i
npm run dev
npm run lint
npm run render
npm run render:founder
```

Useful still checks:

```sh
npm run still -- out/stills/frame-14s.jpg --frame=421 --scale=0.25
npm run still -- out/stills/frame-24s.jpg --frame=720 --scale=0.25
npm run still -- out/stills/frame-33s.jpg --frame=990 --scale=0.25
npm run still:founder -- out/stills/founder-02s.jpg --frame=60 --scale=0.25
npm run still:founder -- out/stills/founder-32s.jpg --frame=960 --scale=0.25
npm run still:founder -- out/stills/founder-34s.jpg --frame=1035 --scale=0.25
npm run still:founder -- out/stills/founder-38s.jpg --frame=1140 --scale=0.25
```
