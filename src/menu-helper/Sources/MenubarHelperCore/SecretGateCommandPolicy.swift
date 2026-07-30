private struct SecretGateCommandPolicy: Sendable {
    let readOnly: Set<String>
    let mutating: Set<String>
    let secretDump: Set<String>

    init(_ readOnly: String, _ mutating: String, secretDump: String = "") {
        self.readOnly = Self.commands(readOnly)
        self.mutating = Self.commands(mutating)
        self.secretDump = Self.commands(secretDump)
    }

    private static func commands(_ value: String) -> Set<String> {
        Set(value.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) })
    }
}

public func genericSecretGateRequestClassification(
    gateID: String,
    arguments: [String]
) -> SecretGateRequestClassification {
    let words = arguments.map { $0.lowercased() }
    guard !words.isEmpty else { return .unknown }
    if words == ["help"] || words == ["--help"] || words == ["version"] || words == ["--version"] {
        return .readOnly
    }
    guard let policy = secretGateCommandPolicies[gateID] else { return .unknown }
    let candidates = (1...min(3, words.count)).reversed().map {
        words.prefix($0).joined(separator: " ")
    }
    if candidates.contains(where: policy.secretDump.contains) { return .secretDump }
    if candidates.contains(where: policy.readOnly.contains) { return .readOnly }
    if candidates.contains(where: policy.mutating.contains) { return .mutating }
    return .unknown
}

private let secretGateCommandPolicies: [String: SecretGateCommandPolicy] = [
    "akamai": .init("config list", "config set,config remove", secretDump: "config show"),
    "algolia": .init("profile list", "objects import,objects delete,indices delete", secretDump: "profile get"),
    "argocd": .init("app get,app list,app diff,cluster get,cluster list,account get-user-info", "app create,app set,app sync,app delete,app rollback", secretDump: "account generate-token"),
    "ast-cli": .init("scan list,scan show,project list,project show", "scan create,scan cancel,project create,project delete"),
    "buf": .init("repository list,module list,organization list", "push,repository create,repository delete"),
    "censys": .init("search,view,account", "asm seeds add,asm seeds delete"),
    "checkov": .init("frameworks", "submit"),
    "circleci": .init("project list,pipeline list,config validate", "pipeline run,context create,context delete,context store-secret"),
    "civo": .init("instance list,instance show,kubernetes list,kubernetes show", "instance create,instance remove,kubernetes create,kubernetes remove", secretDump: "apikey show"),
    "cloudsmith-cli": .init("whoami,repos list,packages list,packages search", "push,packages delete,repos create,repos delete"),
    "composer": .init("show,search,outdated,audit,diagnose", "install,update,require,remove,publish", secretDump: "config --auth,config --global --auth"),
    "doctl": .init("account get,compute droplet list,compute droplet get,kubernetes cluster list,kubernetes cluster get", "compute droplet create,compute droplet delete,kubernetes cluster create,kubernetes cluster delete"),
    "flyctl": .init("status,apps list,machine list,machine status,secrets list,auth whoami", "deploy,scale,apps create,apps destroy,machine run,machine destroy,secrets set,secrets unset,secrets import", secretDump: "auth token"),
    "glab": .init("repo view,repo list,issue list,issue view,mr list,mr view,pipeline list,pipeline view", "repo create,repo delete,issue create,mr create,pipeline run", secretDump: "auth token,auth status --show-token"),
    "gotify": .init("health,version", "push"),
    "gptcommit": .init("", "prepare,commit"),
    "grafanactl": .init("resources get,resources list", "resources create,resources delete,resources apply"),
    "heroku": .init("apps,apps info,ps,addons", "apps create,apps destroy,config set,config unset,ps scale", secretDump: "auth token,config"),
    "hcloud": .init("server list,server describe,network list,network describe", "server create,server delete,network create,network delete"),
    "huggingface-cli": .init("auth whoami,repo list,cache scan", "upload,upload-large-folder,repo create,repo delete"),
    "jfrog-cli": .init("rt search,rt ping,rt build-info", "rt upload,rt delete,rt build-publish", secretDump: "config show,config export"),
    "k6": .init("inspect", "run,cloud"),
    "luarocks": .init("search,show,list,which", "install,remove,upload,publish"),
    "minio-mc": .init("ls,stat,find,du,tree", "cp,mv,rm,mb,rb,mirror", secretDump: "alias export"),
    "netlify-cli": .init("status,sites list,functions list", "deploy,sites create,sites delete,functions create", secretDump: "env list,env get"),
    "node": .init("view,info,search,audit,outdated,ping,whoami", "publish,unpublish,deprecate,access,token,dist-tag", secretDump: "config get"),
    "pnpm": .init("view,info,search,audit,outdated,why,list", "publish,unpublish,deprecate,add,remove,update", secretDump: "config get"),
    "pulumi": .init("whoami,stack ls,preview,about,config get", "up,destroy,refresh,import,cancel", secretDump: "config get --show-secrets,stack export --show-secrets"),
    "qwen-code": .init("", "chat,run"),
    "runpodctl": .init("get,list", "create,remove,start,stop", secretDump: "config view"),
    "s3cmd": .init("ls,la,info,du", "put,get,del,rm,sync,cp,mv,mb,rb", secretDump: "--dump-config"),
    "sentry-cli": .init("projects list,organizations list,releases list", "send-event,releases new,releases deploys new,upload-dif"),
    "snowflake-cli": .init("object list,object describe,connection test", "object create,object drop,stage copy"),
    "snyk": .init("", "monitor,auth"),
    "transifex-cli": .init("status", "pull,push"),
    "travis": .init("whoami,repos,history,show,logs", "restart,cancel,enable,disable", secretDump: "token"),
    "twine": .init("check", "upload"),
    "vagrant": .init("status,global-status,validate,version", "up,destroy,halt,reload,suspend,resume,cloud publish"),
    "vault": .init("status,list,kv list,token lookup", "write,delete,kv put,kv delete", secretDump: "read,kv get,login,token create,token generate"),
    "virustotal-cli": .init("file,url,domain,ip,collection", "scan,upload"),
    "vultr": .init("instance list,instance get,region list,plan list", "instance create,instance delete,instance start,instance stop"),
    "wsk": .init("action list,action get,namespace list,package list,trigger list", "action create,action update,action delete,action invoke", secretDump: "property get"),
    "stripe": .init("get,customers list,customers retrieve,products list,products retrieve,prices list,prices retrieve", "post,delete,trigger"),
    "supabase": .init("projects list,functions list,status,inspect", "link,unlink,db push,db reset,functions deploy,secrets set,secrets unset"),
]

let genericSecretGatePolicyIDs: Set<String> = Set(secretGateCommandPolicies.keys)
