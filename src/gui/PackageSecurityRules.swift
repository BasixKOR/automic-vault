import Foundation

enum PackageSecurityRules {
    static func shouldConvertRadioisotope(
        detail: PackageDetail,
        plan: NucleusBridge.IsotopeMigrationPlan
    ) -> Bool {
        guard plan.isRadioisotope == true,
              detail.installed,
              detail.isHomebrewInstall == false,
              let modifiesPackage = plan.modifiesPackage else {
            return false
        }
        return detail.helperPackageNames.contains(modifiesPackage)
    }
}
