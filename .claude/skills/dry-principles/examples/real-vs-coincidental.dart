// DRY — Real vs Coincidental Similarity — Dart/Flutter Examples
// "Do these change for the same reason?"

// --- Scenario 1: Real Duplication (Extract) ---

// BAD — Same fee calculation in two cubits
// When the fee structure changes, BOTH must update.
class PaymentNotifier extends Notifier<PaymentState> {
  Future<void> initiatePayment(double amount) async {
    var fee = amount * 0.015; // Fee calculation — copy 1
    if (amount > 10000) {
      fee = amount * 0.01;   // Volume discount — copy 1
    }

    final payment = Payment(amount: amount, fee: fee);
    // ...
  }
}

class TransferNotifier extends Notifier<TransferState> {
  Future<void> initiateTransfer(double amount) async {
    var fee = amount * 0.015; // Fee calculation — copy 2
    if (amount > 10000) {
      fee = amount * 0.01;   // Volume discount — copy 2
    }

    final transfer = Transfer(amount: amount, fee: fee);
    // ...
  }
}

// GOOD — Extracted to a single source of truth
class FeeCalculator {
  double calculate(double amount) {
    final rate = amount > 10000 ? 0.01 : 0.015;
    return amount * rate;
  }
}

class PaymentNotifier extends Notifier<PaymentState> {
  final FeeCalculator _feeCalculator;

  PaymentNotifier(this._feeCalculator) : super(const PaymentState.initial());

  Future<void> initiatePayment(double amount) async {
    final fee = _feeCalculator.calculate(amount);
    final payment = Payment(amount: amount, fee: fee);
    // ...
  }
}

class TransferNotifier extends Notifier<TransferState> {
  final FeeCalculator _feeCalculator;

  TransferNotifier(this._feeCalculator) : super(const TransferState.initial());

  Future<void> initiateTransfer(double amount) async {
    final fee = _feeCalculator.calculate(amount);
    final transfer = Transfer(amount: amount, fee: fee);
    // ...
  }
}

// --- Scenario 2: Coincidental Similarity (Keep Separate) ---

// GOOD — These look similar but represent different business concepts
@freezed
class PaymentFormState with _$PaymentFormState {
  const factory PaymentFormState({
    required double amount,
    required String currency,
    required String description,
    required String walletId,
  }) = _PaymentFormState;
}

@freezed
class TransferFormState with _$TransferFormState {
  const factory TransferFormState({
    required double amount,
    required String currency,
    required String description,
    required String sourceWalletId,    // Already different
    required String destinationWalletId, // Transfer-specific
  }) = _TransferFormState;
}

// BAD — Premature extraction: "They both have amount, currency, description!"
@freezed
class TransactionFormState with _$TransactionFormState {
  // "SharedHelper" smell — too generic a name
  const factory TransactionFormState({
    required double amount,
    required String currency,
    required String description,
  }) = _TransactionFormState;
}
// When PaymentFormState adds recurringSchedule and TransferFormState adds
// destinationWalletId, this shared state becomes a wrong abstraction.
// DRY: Coincidental similarity — payments and transfers evolve independently

// --- Scenario 3: Wrong Abstraction (Inline Back) ---

// BAD — Base cubit accumulating behavior to serve different screens
abstract class BaseTransactionNotifier<T> extends Notifier<T> {
  final bool isTransfer;        // Boolean flag
  final bool requiresApproval;  // Boolean flag

  BaseTransactionNotifier(
    super.initialState, {
    required this.isTransfer,
    required this.requiresApproval,
  });

  Future<void> processTransaction(double amount) async {
    if (isTransfer) {
      await _validateTransferRules(amount);
    } else {
      await _validatePaymentRules(amount);
    }

    final fee = isTransfer
        ? _calculateTransferFee(amount)
        : _calculatePaymentFee(amount);

    if (requiresApproval) {
      await _submitForApproval(amount, fee);
    } else {
      await _executeImmediately(amount, fee);
    }
  }

  // Subclasses must implement — but they only use half the methods
  Future<void> _validateTransferRules(double amount);
  Future<void> _validatePaymentRules(double amount);
  // ...
}

// GOOD — Inlined back to separate, clear cubits
class PaymentNotifier extends Notifier<PaymentState> {
  final FeeCalculator _feeCalculator;
  final PaymentRepository _paymentRepository;

  PaymentNotifier(this._feeCalculator, this._paymentRepository)
      : super(const PaymentState.initial());

  Future<void> processPayment(double amount) async {
    _validatePaymentRules(amount);
    final fee = _feeCalculator.calculate(amount);
    await _executePayment(amount, fee);
  }

  // ...
}

class TransferNotifier extends Notifier<TransferState> {
  final FeeCalculator _feeCalculator;
  final TransferRepository _transferRepository;

  TransferNotifier(this._feeCalculator, this._transferRepository)
      : super(const TransferState.initial());

  Future<void> processTransfer(double amount) async {
    _validateTransferRules(amount);
    final fee = _feeCalculator.calculate(amount);

    if (_requiresApproval(amount)) {
      await _submitForApproval(amount, fee);
    } else {
      await _executeTransfer(amount, fee);
    }
  }

  // ...
}

// FeeCalculator is still shared — that's REAL knowledge duplication.
// The base cubit was coincidental similarity that diverged.
// DRY: Inlined wrong abstraction — fee calculation remains shared (real knowledge),
// processing workflows separated (coincidental similarity)
