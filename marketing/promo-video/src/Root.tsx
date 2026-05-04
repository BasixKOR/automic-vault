import "./index.css";
import { Composition } from "remotion";
import { MyComposition, durationInFrames } from "./Composition";
import { FounderStory, founderStoryDurationInFrames } from "./FounderStory";

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
      <Composition
        id="AutomicVaultFounderStory"
        component={FounderStory}
        durationInFrames={founderStoryDurationInFrames}
        fps={30}
        width={1920}
        height={1080}
      />
    </>
  );
};
