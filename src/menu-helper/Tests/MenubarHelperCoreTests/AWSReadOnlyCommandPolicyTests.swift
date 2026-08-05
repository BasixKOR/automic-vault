import Testing
@testable import MenubarHelperCore

@Test(arguments: [
    ("cloudfront", "get-distribution"),
    ("cloudfront", "get-distribution-config"),
    ("dynamodb", "batch-get-item"),
    ("dynamodb", "get-item"),
    ("dynamodb", "query"),
    ("ec2", "describe-instances"),
    ("resource-explorer-2", "search"),
    ("s3", "ls"),
    ("s3api", "head-object"),
    ("sts", "get-caller-identity"),
])
func knownAWSReadsAreApproved(service: String, operation: String) {
    #expect(awsCommandIsReadOnly(service: service, operation: operation))
}

@Test(arguments: [
    ("bedrock-runtime", "invoke-model"),
    ("cloudformation", "detect-stack-drift"),
    ("cloudfront", "future-get-operation"),
    ("cognito-idp", "list-user-pool-client-secrets"),
    ("cognito-idp", "get-user-attribute-verification-code"),
    ("datazone", "list-connections"),
    ("ecr", "get-authorization-token"),
    ("ecr", "get-login-password"),
    ("lambda", "get-function"),
    ("s3", "presign"),
    ("s3api", "get-object"),
    ("secretsmanager", "get-secret-value"),
    ("sqs", "receive-message"),
    ("ssm", "get-parameter"),
    ("sts", "get-session-token"),
    ("storagegateway", "describe-chap-credentials"),
])
func sensitiveStatefulAndUnknownAWSCommandsStayGated(service: String, operation: String) {
    #expect(!awsCommandIsReadOnly(service: service, operation: operation))
}
