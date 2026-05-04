import "./index.css";
import { Composition } from "remotion";
import { FounderStory, founderStoryDurationInFrames } from "./FounderStory";

export const RemotionRoot: React.FC = () => {
  return (
    <Composition
      id="AutomicVaultFounderStory"
      component={FounderStory}
      durationInFrames={founderStoryDurationInFrames}
      fps={30}
      width={1920}
      height={1080}
    />
  );
};
