import Foundation
import Testing
@testable import MenubarHelperCore

@Test func parsesOnlyDefaultBackedRoleChains() throws {
    let chain = try AWSProfileChain.parse("""
    [default]
    region = us-west-2
    [profile dev]
    source_profile = default
    role_arn = arn:aws:iam::123456789012:role/dev
    mfa_serial = arn:aws:iam::123456789012:mfa/max
    """, selectedProfile: "dev")
    #expect(chain.profiles.map(\.name) == ["default", "dev"])
    #expect(chain.region == "us-west-2")
    #expect(chain.selected.roleARN == "arn:aws:iam::123456789012:role/dev")
}

@Test func rejectsAmbientCredentialProviders() {
    #expect(throws: AWSCredentialError.self) {
        try AWSProfileChain.parse("[default]\ncredential_process = steal\n", selectedProfile: "default")
    }
    #expect(throws: AWSCredentialError.self) {
        try AWSProfileChain.parse("[profile dev]\nregion=us-east-1\n", selectedProfile: "dev")
    }
    #expect(throws: AWSCredentialError.self) {
        try AWSProfileChain.parse("[default]\nmfa_process = unsafe\n", selectedProfile: "default")
    }
}

@Test func runtimeBindingRequiresInterpreterAndExactArguments() {
    let interpreter = "/opt/homebrew/Cellar/python@3.14/3.14.6/Frameworks/Python.framework/Versions/3.14/bin/python3.14"
    let process = "/opt/homebrew/Cellar/python@3.14/3.14.6/Frameworks/Python.framework/Versions/3.14/Resources/Python.app/Contents/MacOS/Python"
    let target = "/opt/homebrew/bin/aws"
    let arguments = ["s3", "ls"]
    #expect(awsRuntimeMatches(
        generation: .homebrewV1,
        interpreter: interpreter,
        processPath: process,
        processArguments: [process, target] + arguments,
        target: target,
        approvedArguments: arguments
    ))
    #expect(!awsRuntimeMatches(
        generation: .homebrewV1,
        interpreter: interpreter,
        processPath: process,
        processArguments: [process, target, "iam", "list-users"],
        target: target,
        approvedArguments: arguments
    ))
}

@Test func officialRuntimeBindingRequiresTheNativeTargetAndExactArguments() {
    let target = "/opt/av/aws/current/aws"
    #expect(awsRuntimeMatches(
        generation: .officialV2,
        interpreter: target,
        processPath: target,
        processArguments: [target, "s3", "ls"],
        target: target,
        approvedArguments: ["s3", "ls"]
    ))
    #expect(!awsRuntimeMatches(
        generation: .officialV2,
        interpreter: target,
        processPath: "/opt/homebrew/bin/aws",
        processArguments: ["/opt/homebrew/bin/aws", "s3", "ls"],
        target: target,
        approvedArguments: ["s3", "ls"]
    ))
}

@Test func helperNegotiationAndStubGenerationPreserveLegacyButFailClosedAfterUpgrade() {
    #expect(negotiatedAWSHelperProtocolVersion(requested: 0) == 1)
    #expect(negotiatedAWSHelperProtocolVersion(requested: 2) == 2)
    #expect(negotiatedAWSHelperProtocolVersion(requested: 1) == nil)
    #expect(awsGenerationMatchesInstalledStub(
        .homebrewV1,
        target: AWSRuntimeGeneration.homebrewV1.target,
        stub: AWSRuntimeGeneration.homebrewV1.stub
    ))
    #expect(awsGenerationMatchesInstalledStub(
        .officialV2,
        target: AWSRuntimeGeneration.officialV2.target,
        stub: AWSRuntimeGeneration.officialV2.stub
    ))
    #expect(!awsGenerationMatchesInstalledStub(
        .homebrewV1,
        target: AWSRuntimeGeneration.homebrewV1.target,
        stub: AWSRuntimeGeneration.officialV2.stub
    ))
}

@Test func parsesOnlyArgumentFreeAWSInterpreters() throws {
    #expect(try awsInterpreter(fromShebang: "#!/opt/homebrew/bin/python3") == "/opt/homebrew/bin/python3")
    for shebang in ["python3", "#!/usr/bin/env python3", "#!/opt/homebrew/bin/python3 -S"] {
        #expect(throws: AWSCredentialError.unsupportedRuntime(
            "the AWS CLI shebang must contain one absolute interpreter without arguments"
        )) {
            try awsInterpreter(fromShebang: shebang)
        }
    }
}

@Test func signsSTSRequestsDeterministically() throws {
    let date = try #require(ISO8601DateFormatter().date(from: "2015-08-30T12:36:00Z"))
    let request = try awsSTSRequest(
        region: "us-east-1",
        parameters: ["Action": "GetSessionToken", "Version": "2011-06-15"],
        credentials: AWSCredentials(accessKeyID: "AKIDEXAMPLE", secretAccessKey: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY"),
        date: date
    )
    #expect(String(decoding: request.body, as: UTF8.self) == "Action=GetSessionToken&Version=2011-06-15")
    #expect(request.headers["authorization"]?.contains("Credential=AKIDEXAMPLE/20150830/us-east-1/sts/aws4_request") == true)
    #expect(request.headers["authorization"]?.hasSuffix("Signature=90632c13626f68af39ea69ccb0f339aa612c376059d1593c15b4a621da547efa") == true)
}

@Test func parsesSTSCredentials() throws {
    let credentials = try parseAWSTSCredentials(Data("""
    <GetSessionTokenResponse><GetSessionTokenResult><Credentials>
    <AccessKeyId>ASIAEXAMPLE</AccessKeyId><SecretAccessKey>secret</SecretAccessKey>
    <SessionToken>token</SessionToken><Expiration>2026-08-05T18:00:00Z</Expiration>
    </Credentials></GetSessionTokenResult></GetSessionTokenResponse>
    """.utf8))
    #expect(credentials.accessKeyID == "ASIAEXAMPLE")
    #expect(credentials.sessionToken == "token")
}
