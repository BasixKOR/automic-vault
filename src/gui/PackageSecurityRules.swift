import Foundation

enum PackageSecurityRules {
    static func shouldConvertRadioisotope(
        detail: PackageDetail,
        plan: NucleusBridge.IsotopeMigrationPlan
    ) -> Bool {
        guard plan.isRadioisotope == true,
              detail.installed,
              let modifiesPackage = plan.modifiesPackage else {
            return false
        }
        return detail.helperPackageNames.contains { helperPackageName in
            packageName(helperPackageName, matchesModifiedPackage: modifiesPackage)
        }
    }

    private static func packageName(_ packageName: String, matchesModifiedPackage modified: String) -> Bool {
        let packageName = normalizedPackageName(packageName)
        let modified = normalizedPackageName(modified)
        if packageName == modified {
            return true
        }
        guard let base = versionedFormulaBase(packageName) else {
            return false
        }
        return base == modified
    }

    private static func normalizedPackageName(_ packageName: String) -> String {
        let trimmed = packageName.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return trimmed.strippingPrefix("brew:") ?? trimmed
    }

    private static func versionedFormulaBase(_ formula: String) -> String? {
        guard let separator = formula.lastIndex(of: "@") else {
            return nil
        }
        let base = formula[..<separator]
        let version = formula[formula.index(after: separator)...]
        guard !base.isEmpty,
              !version.isEmpty,
              version.unicodeScalars.allSatisfy({ scalar in
                  scalar.value >= 48 && scalar.value <= 57
              }) else {
            return nil
        }
        return String(base)
    }
}
