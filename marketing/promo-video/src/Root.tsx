import "./index.css";
import { Composition } from "remotion";
import {
  BrewInstallSecurityComposition,
  brewInstallSecurityDurationInFrames,
} from "./BrewInstallSecurity";
import { MyComposition, durationInFrames } from "./Composition";
import { SecretSkillsComposition, secretSkillsDurationInFrames } from "./SecretSkills";

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
        id="AutomicVaultSkillSecrets"
        component={SecretSkillsComposition}
        durationInFrames={secretSkillsDurationInFrames}
        fps={30}
        width={1920}
        height={1080}
      />
      <Composition
        id="AutomicVaultBrewInstallSecurity"
        component={BrewInstallSecurityComposition}
        durationInFrames={brewInstallSecurityDurationInFrames}
        fps={30}
        width={1920}
        height={1080}
      />
    </>
  );
};
