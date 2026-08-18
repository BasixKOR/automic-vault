import Observation
import StoreKit

@MainActor @Observable
final class ApprovalSubscription {
    static let shared = ApprovalSubscription()
    static let annualProductID = "com.automicvault.approval.annual"
    static let monthlyProductID = "com.automicvault.approval.monthly"
    static let productIDs = [annualProductID, monthlyProductID]

    enum State: Equatable {
        case loading
        case inactive
        case active
    }

    private(set) var state: State = .loading
    private(set) var productsAvailable = false
    var errorMessage: String?

    private var transactionUpdates: Task<Void, Never>?

    private init() {
        transactionUpdates = Task { [weak self] in
            for await result in Transaction.updates {
                guard !Task.isCancelled else { return }
                guard case .verified(let transaction) = result else {
                    await self?.refresh()
                    continue
                }
                await self?.handle(transaction)
            }
        }
    }

    func start() async {
        do {
            let products = try await Product.products(for: Self.productIDs)
            productsAvailable = Set(products.map(\.id)) == Set(Self.productIDs)
            if !productsAvailable {
                errorMessage = "iPhone Approval subscriptions are not available from the App Store."
            }
        } catch {
            productsAvailable = false
            errorMessage = "The App Store could not load iPhone Approval subscriptions."
        }
        await refresh()
    }

    @discardableResult
    func refresh() async -> Bool {
        let now = Date()
        var active = false

        for productID in Self.productIDs {
            for await result in Transaction.currentEntitlements(for: productID) {
                guard case .verified(let transaction) = result,
                      transaction.revocationDate == nil,
                      !transaction.isUpgraded,
                      transaction.expirationDate.map({ $0 > now }) ?? false else { continue }
                active = true
            }
        }

        state = active ? .active : .inactive
        return active
    }

    func handlePurchase(_ result: Result<Product.PurchaseResult, any Error>) async {
        switch result {
        case .success(.success(let verification)):
            guard case .verified(let transaction) = verification else {
                errorMessage = "The App Store purchase could not be verified."
                await refresh()
                return
            }
            await handle(transaction)
            return
        case .success(.pending):
            errorMessage = "The purchase is pending approval from the App Store."
        case .success(.userCancelled):
            break
        case .failure:
            errorMessage = "The App Store could not complete the purchase."
        @unknown default:
            errorMessage = "The App Store returned an unknown purchase result."
        }
        await refresh()
    }

    private func handle(_ transaction: Transaction) async {
        await transaction.finish()
        guard Self.productIDs.contains(transaction.productID),
              transaction.revocationDate == nil,
              !transaction.isUpgraded,
              transaction.expirationDate.map({ $0 > .now }) ?? false else {
            await refresh()
            return
        }
        state = .active
    }
}
