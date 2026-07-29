// OCP — Open/Closed Principle — Dart/Flutter Examples
// Open for extension, closed for modification.

// --- Example 1: Payment status handling with if-else chain ---

// BAD — Adding a new status requires modifying this widget
class PaymentStatusBadge extends StatelessWidget {
  final PaymentStatus status;

  const PaymentStatusBadge({super.key, required this.status});

  @override
  Widget build(BuildContext context) {
    // Growing if-else chain — new status = modify this widget
    if (status == PaymentStatus.pending) {
      return _badge('Pending', Colors.orange, Icons.schedule);
    } else if (status == PaymentStatus.completed) {
      return _badge('Completed', Colors.green, Icons.check_circle);
    } else if (status == PaymentStatus.failed) {
      return _badge('Failed', Colors.red, Icons.error);
    } else if (status == PaymentStatus.refunded) {
      return _badge('Refunded', Colors.blue, Icons.undo);
    } else {
      return _badge('Unknown', Colors.grey, Icons.help);
    }
  }

  Widget _badge(String label, Color color, IconData icon) {
    return Chip(
      avatar: Icon(icon, color: color, size: 16),
      label: Text(label),
      backgroundColor: color.withOpacity(0.1),
    );
  }
}

// GOOD — Status presentation defined per status using sealed class pattern
sealed class PaymentStatus {
  String get label;
  Color get color;
  IconData get icon;

  const PaymentStatus();
}

class PendingStatus extends PaymentStatus {
  @override String get label => 'Pending';
  @override Color get color => Colors.orange;
  @override IconData get icon => Icons.schedule;
}

class CompletedStatus extends PaymentStatus {
  @override String get label => 'Completed';
  @override Color get color => Colors.green;
  @override IconData get icon => Icons.check_circle;
}

// New status? Add a new class. PaymentStatusBadge unchanged.
class PaymentStatusBadge extends StatelessWidget {
  final PaymentStatus status;

  const PaymentStatusBadge({super.key, required this.status});

  @override
  Widget build(BuildContext context) {
    return Chip(
      avatar: Icon(status.icon, color: status.color, size: 16),
      label: Text(status.label),
      backgroundColor: status.color.withOpacity(0.1),
    );
  }
}

// --- Example 2: Validation rule expansion ---

// BAD — Adding a validation rule requires modifying the validator
class TransferValidator {
  Either<Failure, void> validate(TransferRequest request) {
    if (request.amount <= 0) {
      return const Left(ValidationFailure('Amount must be positive'));
    }
    if (request.amount > 10000) {
      return const Left(ValidationFailure('Amount exceeds limit'));
    }
    if (request.sourceWalletId == request.destinationWalletId) {
      return const Left(ValidationFailure('Cannot transfer to same wallet'));
    }
    // New rule? Modify this method.
    return const Right(null);
  }
}

// GOOD — Rules are composable and extensible
abstract class ValidationRule<T> {
  Either<Failure, void> validate(T input);
}

class PositiveAmountRule extends ValidationRule<TransferRequest> {
  @override
  Either<Failure, void> validate(TransferRequest request) {
    if (request.amount <= 0) {
      return const Left(ValidationFailure('Amount must be positive'));
    }
    return const Right(null);
  }
}

class AmountLimitRule extends ValidationRule<TransferRequest> {
  @override
  Either<Failure, void> validate(TransferRequest request) {
    if (request.amount > 10000) {
      return const Left(ValidationFailure('Amount exceeds limit'));
    }
    return const Right(null);
  }
}

class DifferentWalletsRule extends ValidationRule<TransferRequest> {
  @override
  Either<Failure, void> validate(TransferRequest request) {
    if (request.sourceWalletId == request.destinationWalletId) {
      return const Left(ValidationFailure('Cannot transfer to same wallet'));
    }
    return const Right(null);
  }
}

// Composite validator — new rule = add a new class, register in the list
class CompositeValidator<T> {
  final List<ValidationRule<T>> _rules;

  const CompositeValidator(this._rules);

  Either<Failure, void> validate(T input) {
    for (final rule in _rules) {
      final result = rule.validate(input);
      if (result.isLeft()) return result;
    }
    return const Right(null);
  }
}

// --- Example 3: Data source strategy ---

// BAD — Adding a data source requires modifying the repository
class WalletRepository {
  final WalletApi _api;
  final WalletLocalDb _localDb;

  Future<Either<Failure, Wallet>> getWallet(String id) async {
    try {
      final model = await _api.getWallet(id);
      return Right(model.toDomain());
    } catch (e) {
      // Fallback to local — but adding a third source means modifying this
      final cached = await _localDb.getWallet(id);
      if (cached != null) return Right(cached.toDomain());
      return Left(ServerFailure(message: e.toString()));
    }
  }
}

// GOOD — Data sources are pluggable via strategy pattern
abstract class WalletDataSource {
  Future<WalletModel?> getWallet(String id);
  int get priority;
}

class RemoteWalletDataSource extends WalletDataSource {
  final WalletApi _api;

  @override int get priority => 1;

  @override
  Future<WalletModel?> getWallet(String id) async {
    try {
      return await _api.getWallet(id);
    } catch (_) {
      return null;
    }
  }
}

class LocalWalletDataSource extends WalletDataSource {
  final WalletLocalDb _localDb;

  @override int get priority => 2;

  @override
  Future<WalletModel?> getWallet(String id) => _localDb.getWallet(id);
}
