# Automic Vault Founder Story Video

Remotion source for the Automic Vault founder story video.

The composition is `AutomicVaultFounderStory` at 1920x1080 and 30 fps. The
rendered output is ignored under `out/`.

## Commands

```sh
npm i
npm run dev
npm run lint
npm run render
```

Useful still checks:

```sh
npm run still -- out/stills/founder-opening.jpg --frame=250 --scale=0.25
npm run still -- out/stills/founder-agents.jpg --frame=492 --scale=0.25
npm run still -- out/stills/founder-close.jpg --frame=1140 --scale=0.25
```
