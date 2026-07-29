// DIP — Dependency Inversion Principle — Dart/Flutter Examples
// Depend on abstractions, not concretions.

// --- Example 1: Notifier depending on concrete data source ---

// BAD — Notifier directly depends on concrete HTTP client
@riverpod
class PaymentListNotifier extends _$PaymentListNotifier {
  @override
  Future<List<Payment>> build(String walletId) async {
    final dio = Dio();  // Concrete HTTP client, untestable
    final response = await dio.get('/api/wallets/$walletId/payments');
    return (response.data as List)
        .map((json) => PaymentModel.fromJson(json).toDomain())
        .toList();
  }
}

// GOOD — Notifier depends on abstract API client via provider
@riverpod
class PaymentListNotifier extends _$PaymentListNotifier {
  @override
  Future<List<Payment>> build(String walletId) async {
    final apiClient = ref.watch(paymentApiClientProvider);  // Injected via provider
    return apiClient.getByWalletId(walletId);
  }
}

// Interface defined in domain/application layer
abstract class PaymentRepository {
  Future<Either<Failure, List<Payment>>> getByWalletId(String walletId);
}

// Implementation in data layer
class PaymentRepositoryImpl implements PaymentRepository {
  final PaymentRemoteDataSource _remote;

  @override
  Future<Either<Failure, List<Payment>>> getByWalletId(String walletId) async {
    try {
      final models = await _remote.getPaymentsByWalletId(walletId);
      return Right(models.map((m) => m.toDomain()).toList());
    } on ServerException catch (e) {
      return Left(ServerFailure(message: e.message));
    }
  }
}

// --- Example 2: Direct SharedPreferences access ---

// BAD — Application logic depends directly on SharedPreferences
class UserSettingsService {
  Future<void> saveThemePreference(bool isDarkMode) async {
    // Direct dependency on concrete storage mechanism
    final prefs = await SharedPreferences.getInstance();
    await prefs.setBool('dark_mode', isDarkMode);
  }

  Future<bool> getThemePreference() async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getBool('dark_mode') ?? false;
  }

  Future<void> saveLanguage(String languageCode) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString('language', languageCode);
  }
}

// GOOD — Depend on abstraction for local storage
abstract class LocalStorage {
  Future<void> setBool(String key, bool value);
  Future<bool?> getBool(String key);
  Future<void> setString(String key, String value);
  Future<String?> getString(String key);
}

class UserSettingsService {
  final LocalStorage _storage;

  const UserSettingsService(this._storage);

  Future<void> saveThemePreference(bool isDarkMode) async {
    await _storage.setBool('dark_mode', isDarkMode);
  }

  Future<bool> getThemePreference() async {
    return await _storage.getBool('dark_mode') ?? false;
  }
}

// SharedPreferences implementation in infrastructure
class SharedPreferencesStorage implements LocalStorage {
  final SharedPreferences _prefs;

  const SharedPreferencesStorage(this._prefs);

  @override
  Future<void> setBool(String key, bool value) => _prefs.setBool(key, value);

  @override
  Future<bool?> getBool(String key) async => _prefs.getBool(key);

  // ... other methods
}

// --- Example 3: Direct platform channel access ---

// BAD — Feature code directly calls platform-specific APIs
class BiometricAuthService {
  Future<bool> authenticate() async {
    // Direct dependency on concrete plugin
    final localAuth = LocalAuthentication();
    final canAuth = await localAuth.canCheckBiometrics;
    if (!canAuth) return false;

    return await localAuth.authenticate(
      localizedReason: 'Authenticate to access your wallet',
      options: const AuthenticationOptions(
        biometricOnly: true,
        stickyAuth: true,
      ),
    );
  }
}

// GOOD — Abstract the biometric capability
abstract class BiometricAuthenticator {
  Future<bool> isAvailable();
  Future<Either<Failure, bool>> authenticate({required String reason});
}

class BiometricAuthService {
  final BiometricAuthenticator _authenticator;

  const BiometricAuthService(this._authenticator);

  Future<Either<Failure, bool>> authenticateForWalletAccess() async {
    final available = await _authenticator.isAvailable();
    if (!available) return const Left(BiometricNotAvailableFailure());

    return _authenticator.authenticate(
      reason: 'Authenticate to access your wallet',
    );
  }
}

// Platform implementation in infrastructure
class LocalAuthBiometricAuthenticator implements BiometricAuthenticator {
  final LocalAuthentication _localAuth;

  const LocalAuthBiometricAuthenticator(this._localAuth);

  @override
  Future<bool> isAvailable() => _localAuth.canCheckBiometrics;

  @override
  Future<Either<Failure, bool>> authenticate({required String reason}) async {
    try {
      final result = await _localAuth.authenticate(
        localizedReason: reason,
        options: const AuthenticationOptions(biometricOnly: true),
      );
      return Right(result);
    } catch (e) {
      return Left(BiometricFailure(message: e.toString()));
    }
  }
}
