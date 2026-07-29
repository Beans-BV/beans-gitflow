// SRP — Single Responsibility Principle — Dart/Flutter Examples
// Each class should have only one reason to change.

// --- Example 1: Notifier doing too much ---

// BAD — WalletNotifier handles state, validation, formatting, AND navigation
@riverpod
class WalletNotifier extends _$WalletNotifier {
  @override
  Future<WalletState> build(String walletId) async {
    // Validation concern
    if (walletId.isEmpty) {
      throw ArgumentError('Invalid wallet ID');
    }

    final apiClient = ref.watch(walletApiClientProvider);
    final result = await apiClient.getWallet(walletId);

    // Navigation concern — notifier shouldn't know about routes
    if (result.requiresKyc) {
      ref.read(routerProvider).push('/kyc');
    }

    return WalletState.loaded(result);
  }
}

// GOOD — Notifier manages state only; other concerns separated
@riverpod
class WalletNotifier extends _$WalletNotifier {
  @override
  Future<Wallet> build(String walletId) async {
    final apiClient = ref.watch(walletApiClientProvider);
    return apiClient.getWallet(walletId);
  }
}
// Navigation handled via ref.listen in the widget
// Error messages handled in the presentation layer via AsyncValue.when

// --- Example 2: Repository with mixed concerns ---

// BAD — Repository does caching, connectivity check, and data mapping
class WalletRepositoryImpl implements WalletRepository {
  final WalletRemoteDataSource _remote;
  final WalletLocalDataSource _local;
  final ConnectivityChecker _connectivity;

  @override
  Future<Either<Failure, Wallet>> getWallet(String id) async {
    // Connectivity concern
    if (!await _connectivity.hasConnection) {
      final cached = await _local.getCachedWallet(id);
      if (cached != null) return Right(cached.toDomain());
      return const Left(NetworkFailure());
    }

    try {
      final model = await _remote.getWallet(id);
      // Caching concern mixed with data access
      await _local.cacheWallet(model);
      // Mapping concern — toDomain() is fine but mixed with the rest
      return Right(model.toDomain());
    } catch (e) {
      return Left(ServerFailure(message: e.toString()));
    }
  }
}

// GOOD — Repository focuses on data coordination
class WalletRepositoryImpl implements WalletRepository {
  final WalletRemoteDataSource _remote;

  @override
  Future<Either<Failure, Wallet>> getWallet(String id) async {
    try {
      final model = await _remote.getWallet(id);
      return Right(model.toDomain());
    } on ServerException catch (e) {
      return Left(ServerFailure(message: e.message));
    }
  }
}
// Caching handled at the data source level or via a caching decorator
// Connectivity handled by the HTTP client or interceptor

// --- Example 3: Domain entity with formatting ---

// BAD — Entity formats its own display values
class Payment {
  final String id;
  final double amount;
  final String currency;
  final DateTime createdAt;

  // Display concern — domain shouldn't format for UI
  String get formattedAmount => '${currency} ${amount.toStringAsFixed(2)}';

  // Date formatting — UI concern
  String get formattedDate =>
      '${createdAt.day}/${createdAt.month}/${createdAt.year}';
}

// GOOD — Entity contains only domain state and behavior
class Payment {
  final String id;
  final Money amount;
  final DateTime createdAt;

  bool get isPending => status == PaymentStatus.pending;
  bool get canBeRefunded => status == PaymentStatus.completed &&
      DateTime.now().difference(createdAt).inDays <= 30;
}
// Formatting lives in presentation layer (extensions, formatters, or widgets)
