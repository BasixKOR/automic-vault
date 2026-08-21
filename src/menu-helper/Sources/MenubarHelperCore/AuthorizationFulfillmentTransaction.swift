/// Keeps staged authorization material private until its record has persisted.
package struct AuthorizationFulfillmentTransaction<Material> {
    private let material: Material

    package init(material: Material) {
        self.material = material
    }

    @discardableResult
    package func commit(
        record: () -> Bool,
        activate: (Material) -> Void,
        observe: (Material) -> Void,
        release: (Material) -> Void
    ) -> Bool {
        guard record() else { return false }
        activate(material)
        observe(material)
        release(material)
        return true
    }
}
