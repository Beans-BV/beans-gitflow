// LSP — Liskov Substitution Principle — Dart/Flutter Examples
// Subtypes must be substitutable for their base types.

// --- Example 1: Repository with inconsistent error handling ---

// BAD — Implementations return different types of Failures for the same scenario
abstract class WalletRepository {
  Future<Either<Failure, Wallet>> getWallet(String id);
}

class RemoteWalletRepository implements WalletRepository {
  @override
  Future<Either<Failure, Wallet>> getWallet(String id) async {
    try {
      final response = await _api.getWallet(id);
      return Right(response.toDomain());
    } catch (e) {
      return Left(ServerFailure(message: e.toString()));
    }
  }
}

class LocalWalletRepository implements WalletRepository {
  @override
  Future<Either<Failure, Wallet>> getWallet(String id) async {
    final data = await _db.query('wallets', where: 'id = ?', whereArgs: [id]);
    if (data.isEmpty) {
      // LSP violation — throws instead of returning Left
      throw WalletNotFoundException(id);
    }
    return Right(WalletModel.fromMap(data.first).toDomain());
  }
}

// GOOD — Both return Either consistently
class RemoteWalletRepository implements WalletRepository {
  @override
  Future<Either<Failure, Wallet>> getWallet(String id) async {
    try {
      final response = await _api.getWallet(id);
      return Right(response.toDomain());
    } on ServerException catch (e) {
      return Left(ServerFailure(message: e.message));
    }
  }
}

class LocalWalletRepository implements WalletRepository {
  @override
  Future<Either<Failure, Wallet>> getWallet(String id) async {
    final data = await _db.query('wallets', where: 'id = ?', whereArgs: [id]);
    if (data.isEmpty) {
      return Left(NotFoundFailure(entity: 'Wallet', id: id));
    }
    return Right(WalletModel.fromMap(data.first).toDomain());
  }
}

// --- Example 2: Unimplemented methods in sealed class hierarchy ---

// BAD — Some subtypes can't fulfill the contract
sealed class PaymentMethod {
  Future<PaymentResult> process(double amount);
  Future<void> refund(String transactionId);  // Not all methods support refund
}

class CreditCardPayment extends PaymentMethod {
  @override
  Future<PaymentResult> process(double amount) async {
    return await _stripeClient.charge(amount);
  }

  @override
  Future<void> refund(String transactionId) async {
    await _stripeClient.refund(transactionId);
  }
}

class CryptoPayment extends PaymentMethod {
  @override
  Future<PaymentResult> process(double amount) async {
    return await _stellarSdk.sendPayment(amount);
  }

  @override
  Future<void> refund(String transactionId) {
    // LSP violation — crypto payments can't be refunded
    throw UnsupportedError('Crypto payments cannot be refunded');
  }
}

// GOOD — Separate refundable from non-refundable
sealed class PaymentMethod {
  Future<PaymentResult> process(double amount);
}

// Only refundable methods implement this
abstract class RefundablePaymentMethod extends PaymentMethod {
  Future<void> refund(String transactionId);
}

class CreditCardPayment extends RefundablePaymentMethod {
  @override
  Future<PaymentResult> process(double amount) async {
    return await _stripeClient.charge(amount);
  }

  @override
  Future<void> refund(String transactionId) async {
    await _stripeClient.refund(transactionId);
  }
}

class CryptoPayment extends PaymentMethod {
  @override
  Future<PaymentResult> process(double amount) async {
    return await _stellarSdk.sendPayment(amount);
  }
  // No refund method — not part of the contract for this type
}

// --- Example 3: Type checking in the presentation layer ---

// BAD — Widget checks concrete type to decide rendering
class TransactionTile extends StatelessWidget {
  final Transaction transaction;

  const TransactionTile({super.key, required this.transaction});

  @override
  Widget build(BuildContext context) {
    // LSP violation — widget knows about concrete types
    if (transaction is DepositTransaction) {
      return ListTile(
        leading: const Icon(Icons.arrow_downward, color: Colors.green),
        title: Text('+${transaction.amount}'),
        subtitle: Text((transaction as DepositTransaction).source),
      );
    } else if (transaction is WithdrawalTransaction) {
      return ListTile(
        leading: const Icon(Icons.arrow_upward, color: Colors.red),
        title: Text('-${transaction.amount}'),
        subtitle: Text((transaction as WithdrawalTransaction).destination),
      );
    }
    return const SizedBox.shrink();
  }
}

// GOOD — Transaction knows how to present itself (polymorphism)
sealed class Transaction {
  double get amount;
  String get description;
  IconData get icon;
  Color get color;
  String get formattedAmount;
}

class DepositTransaction extends Transaction {
  final String source;

  @override IconData get icon => Icons.arrow_downward;
  @override Color get color => Colors.green;
  @override String get formattedAmount => '+${amount.toStringAsFixed(2)}';
  @override String get description => source;
}

class WithdrawalTransaction extends Transaction {
  final String destination;

  @override IconData get icon => Icons.arrow_upward;
  @override Color get color => Colors.red;
  @override String get formattedAmount => '-${amount.toStringAsFixed(2)}';
  @override String get description => destination;
}

class TransactionTile extends StatelessWidget {
  final Transaction transaction;

  const TransactionTile({super.key, required this.transaction});

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: Icon(transaction.icon, color: transaction.color),
      title: Text(transaction.formattedAmount),
      subtitle: Text(transaction.description),
    );
  }
}
