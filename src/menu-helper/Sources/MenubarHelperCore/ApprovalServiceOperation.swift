public enum ApprovalServiceOperation: String, CaseIterable, Sendable {
    case awsHelperVersion = "aws-helper-version"
    case inject
    case keys
    case authorize
    case awsCredentials = "aws-credentials"
    case list
    case save
    case saveIfAbsent = "save-if-absent"
    case bless
    case delete
    case ghSave = "gh-save"
    case ghDelete = "gh-delete"
    case stripeSave = "stripe-save"
    case stripeDelete = "stripe-delete"
}
