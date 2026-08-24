public enum ApprovalServiceOperation: String, CaseIterable, Sendable {
    case awsHelperVersion = "aws-helper-version"
    case dockerHelperVersion = "docker-helper-version"
    case oxideHelperVersion = "oxide-helper-version"
    case terraformHelperVersion = "terraform-helper-version"
    case inject
    case varlock
    case keys
    case authorize
    case gpgSign = "gpg-sign"
    case proxyStart = "proxy-start"
    case awsCredentials = "aws-credentials"
    case dockerGet = "docker-get"
    case oxideGet = "oxide-get"
    case terraformGet = "terraform-get"
    case dockerSave = "docker-save"
    case dockerDelete = "docker-delete"
    case oxideSave = "oxide-save"
    case oxideDelete = "oxide-delete"
    case terraformSave = "terraform-save"
    case terraformDelete = "terraform-delete"
    case list
    case save
    case saveIfAbsentOrEqual = "save-if-absent"
    case bless
    case delete
    case openWindow = "open-window"
    case ghSave = "gh-save"
    case ghDelete = "gh-delete"
    case stripeSave = "stripe-save"
    case stripeDelete = "stripe-delete"
}
