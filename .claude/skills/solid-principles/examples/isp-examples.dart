// ISP — Interface Segregation Principle — Dart/Flutter Examples
// No client should be forced to depend on methods it does not use.

// --- Example 1: Fat repository interface ---

// BAD — All consumers depend on full CRUD even if they only read
abstract class TransactionRepository {
  Future<Either<Failure, Transaction>> getById(String id);
  Future<Either<Failure, List<Transaction>>> getByWalletId(String walletId);
  Future<Either<Failure, List<Transaction>>> search(TransactionFilter filter);
  Future<Either<Failure, Unit>> create(Transaction transaction);
  Future<Either<Failure, Unit>> update(Transaction transaction);
  Future<Either<Failure, Unit>> delete(String id);
  Future<Either<Failure, Unit>> bulkCreate(List<Transaction> transactions);
  Future<Either<Failure, TransactionStats>> getStats(String walletId);
}

// Transaction list screen only needs read + search — depends on all 8 methods
class TransactionListNotifier extends _$TransactionListNotifier {
  // Uses 2 of 8 methods from the full repository
  @override
  Future<List<Transaction>> build(String walletId) async {
    final repository = ref.watch(transactionRepositoryProvider);
    return repository.getByWalletId(walletId);
  }
}

// GOOD — Segregated interfaces
abstract class TransactionReader {
  Future<Either<Failure, Transaction>> getById(String id);
  Future<Either<Failure, List<Transaction>>> getByWalletId(String walletId);
  Future<Either<Failure, List<Transaction>>> search(TransactionFilter filter);
  Future<Either<Failure, TransactionStats>> getStats(String walletId);
}

abstract class TransactionWriter {
  Future<Either<Failure, Unit>> create(Transaction transaction);
  Future<Either<Failure, Unit>> update(Transaction transaction);
  Future<Either<Failure, Unit>> delete(String id);
  Future<Either<Failure, Unit>> bulkCreate(List<Transaction> transactions);
}

// Implementation can implement both
class TransactionRepositoryImpl implements TransactionReader, TransactionWriter {
  // ... all methods
}

// Notifier depends only on what it needs
class TransactionListNotifier extends _$TransactionListNotifier {
  @override
  Future<List<Transaction>> build(String walletId) async {
    final reader = ref.watch(transactionReaderProvider);  // Only read methods
    return reader.getByWalletId(walletId);
  }
}

// --- Example 2: Wallet feature interface ---

// BAD — Interface forces all wallet types to support all features
abstract class WalletService {
  Future<Either<Failure, Wallet>> getWallet(String id);
  Future<Either<Failure, Unit>> deposit(String walletId, double amount);
  Future<Either<Failure, Unit>> withdraw(String walletId, double amount);
  Future<Either<Failure, Unit>> transfer(String from, String to, double amount);
  Future<Either<Failure, Unit>> stake(String walletId, double amount);
  Future<Either<Failure, Unit>> unstake(String walletId, double amount);
  Future<Either<Failure, StakingRewards>> getStakingRewards(String walletId);
}

// Fiat wallet forced to implement staking methods it doesn't support
class FiatWalletService implements WalletService {
  @override
  Future<Either<Failure, Unit>> stake(String walletId, double amount) {
    throw UnimplementedError('Fiat wallets do not support staking');  // ISP violation
  }
  // ...
}

// GOOD — Segregated by capability
abstract class WalletOperations {
  Future<Either<Failure, Wallet>> getWallet(String id);
  Future<Either<Failure, Unit>> deposit(String walletId, double amount);
  Future<Either<Failure, Unit>> withdraw(String walletId, double amount);
  Future<Either<Failure, Unit>> transfer(String from, String to, double amount);
}

abstract class StakingOperations {
  Future<Either<Failure, Unit>> stake(String walletId, double amount);
  Future<Either<Failure, Unit>> unstake(String walletId, double amount);
  Future<Either<Failure, StakingRewards>> getStakingRewards(String walletId);
}

class FiatWalletService implements WalletOperations {
  // Only implements what fiat wallets can do
}

class CryptoWalletService implements WalletOperations, StakingOperations {
  // Implements both — crypto wallets support staking
}

// --- Example 3: Auth interface with mixed concerns ---

// BAD — Auth interface mixes authentication with user management
abstract class AuthService {
  Future<Either<Failure, AuthToken>> login(String email, String password);
  Future<Either<Failure, Unit>> logout();
  Future<Either<Failure, AuthToken>> refreshToken(String refreshToken);
  Future<Either<Failure, User>> register(RegisterRequest request);
  Future<Either<Failure, Unit>> resetPassword(String email);
  Future<Either<Failure, Unit>> updateProfile(UpdateProfileRequest request);
  Future<Either<Failure, User>> getCurrentUser();
}

// Login screen only needs login — depends on 7 methods
class LoginNotifier extends _$LoginNotifier {
  // Uses 1 of 7 methods from AuthService
}

// GOOD — Segregated by concern
abstract class AuthenticationService {
  Future<Either<Failure, AuthToken>> login(String email, String password);
  Future<Either<Failure, Unit>> logout();
  Future<Either<Failure, AuthToken>> refreshToken(String refreshToken);
}

abstract class RegistrationService {
  Future<Either<Failure, User>> register(RegisterRequest request);
}

abstract class UserProfileService {
  Future<Either<Failure, Unit>> resetPassword(String email);
  Future<Either<Failure, Unit>> updateProfile(UpdateProfileRequest request);
  Future<Either<Failure, User>> getCurrentUser();
}

class LoginNotifier extends _$LoginNotifier {
  // Only authentication — no dependency on registration or profile
}
