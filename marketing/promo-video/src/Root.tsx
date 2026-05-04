import "./index.css";
import { Composition } from "remotion";
import { MyComposition, durationInFrames } from "./Composition";

export const RemotionRoot: React.FC = () => {
  return (
    <>
      <Composition
        id="AutomicVaultPromo"
        component={MyComposition}
        durationInFrames={durationInFrames}
        fps={30}
        width={1920}
        height={1080}
      />
    </>
  );
};
