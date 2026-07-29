// DRY — The Wrong Abstraction Lifecycle — Dart/Flutter Example
// "Duplication is far cheaper than the wrong abstraction." — Sandi Metz
//
// Detailed lifecycle: BaseNotifier accumulating behavior to serve different screens.

// === Stage 1: Two Similar Notifiers ===
// WalletOverviewNotifier and TransactionHistoryNotifier both load paginated data.
// Developer sees similarity and extracts a base class.

// Initial code (before extraction) — two separate cubits:
class WalletOverviewNotifier extends Notifier<WalletOverviewState> {
  final WalletRepository _walletRepository;

  WalletOverviewNotifier(this._walletRepository)
      : super(const WalletOverviewState.initial());

  Future<void> loadWallets({bool refresh = false}) async {
    if (refresh) {
      emit(state.copyWith(status: LoadStatus.loading, page: 0));
    }

    final wallets = await _walletRepository.getWallets(
      page: state.page,
      pageSize: 20,
    );

    emit(state.copyWith(
      status: LoadStatus.loaded,
      wallets: refresh ? wallets : [...state.wallets, ...wallets],
      hasMore: wallets.length == 20,
      page: state.page + 1,
    ));
  }
}

class TransactionHistoryNotifier extends Notifier<TransactionHistoryState> {
  final TransactionRepository _transactionRepository;

  TransactionHistoryNotifier(this._transactionRepository)
      : super(const TransactionHistoryState.initial());

  Future<void> loadTransactions({bool refresh = false}) async {
    if (refresh) {
      emit(state.copyWith(status: LoadStatus.loading, page: 0));
    }

    final transactions = await _transactionRepository.getTransactions(
      page: state.page,
      pageSize: 20,
    );

    emit(state.copyWith(
      status: LoadStatus.loaded,
      transactions: refresh ? transactions : [...state.transactions, ...transactions],
      hasMore: transactions.length == 20,
      page: state.page + 1,
    ));
  }
}

// Developer thinks: "These are almost identical! Only the repository call
// and state field name differ. I should extract a base class."

// === Stage 2: The Extraction (Feels Good) ===

abstract class BasePaginatedNotifier<T, S extends BasePaginatedState<T>>
    extends Notifier<S> {
  BasePaginatedNotifier(super.initialState);

  Future<List<T>> fetchPage(int page, int pageSize);
  S updateState(S state, List<T> items, bool refresh);

  Future<void> loadMore({bool refresh = false}) async {
    if (refresh) {
      emit(resetState(state));
    }

    final items = await fetchPage(state.page, 20);

    emit(updateState(
      state,
      refresh ? items : [...state.items, ...items],
      refresh,
    ).copyWith(
      hasMore: items.length == 20,
      page: state.page + 1,
    ));
  }

  S resetState(S state);
}

// Subclasses look clean:
class WalletOverviewNotifier
    extends BasePaginatedNotifier<Wallet, WalletOverviewState> {
  final WalletRepository _walletRepository;
  // ...
  @override
  Future<List<Wallet>> fetchPage(int page, int pageSize)
      => _walletRepository.getWallets(page: page, pageSize: pageSize);
}

// === Stage 3: Requirements Diverge (Cracks Appear) ===

// New requirements:
// - WalletOverview needs filtering by currency
// - TransactionHistory needs date range filtering
// - A new NotificationList screen needs mark-all-as-read

abstract class BasePaginatedNotifier<T, S extends BasePaginatedState<T>>
    extends Notifier<S> {
  // ...

  // Added for filtering — but each screen filters differently
  Map<String, dynamic> get additionalFilters => {};  // Hook #1

  // Added for NotificationList — but only one screen uses it
  Future<void> onAllItemsLoaded() async {}  // Hook #2

  Future<void> loadMore({bool refresh = false}) async {
    if (refresh) {
      emit(resetState(state));
    }

    final items = await fetchPage(state.page, 20, filters: additionalFilters);

    emit(updateState(/* ... */));

    if (!state.hasMore) {
      await onAllItemsLoaded(); // Only NotificationList uses this
    }
  }
}

class WalletOverviewNotifier
    extends BasePaginatedNotifier<Wallet, WalletOverviewState> {
  @override
  Map<String, dynamic> get additionalFilters => {
    if (state.currencyFilter != null) 'currency': state.currencyFilter,
  };
  // ...
}

class TransactionHistoryNotifier
    extends BasePaginatedNotifier<Transaction, TransactionHistoryState> {
  @override
  Map<String, dynamic> get additionalFilters => {
    if (state.startDate != null) 'from': state.startDate!.toIso8601String(),
    if (state.endDate != null) 'to': state.endDate!.toIso8601String(),
  };
  // ...
}

class NotificationListNotifier
    extends BasePaginatedNotifier<AppNotification, NotificationListState> {
  @override
  Future<void> onAllItemsLoaded() async {
    await _notificationRepository.markAllAsRead();
  }
  // ...
}

// === Stage 4: The Decay ===

// More requirements pile up:
// - WalletOverview needs real-time balance updates via WebSocket
// - TransactionHistory needs grouping by date
// - NotificationList needs swipe-to-dismiss
// - A new ContactList needs alphabetical section headers
//
// The base class grows hooks for each screen's unique behavior.
// Nobody understands the full lifecycle anymore.
// Testing requires understanding the base class + all hooks.

// === The Fix: Inline and Re-Extract ===

// Step 1: Inline the base class back into each cubit.
// Step 2: Each cubit is now self-contained and clear.
// Step 3: Extract ONLY the genuinely shared infrastructure.

// GOOD — Each cubit owns its full workflow
class WalletOverviewNotifier extends Notifier<WalletOverviewState> {
  final WalletRepository _walletRepository;

  WalletOverviewNotifier(this._walletRepository)
      : super(const WalletOverviewState.initial());

  Future<void> loadWallets({bool refresh = false}) async {
    if (refresh) {
      emit(state.copyWith(status: LoadStatus.loading, page: 0, wallets: []));
    }

    emit(state.copyWith(status: LoadStatus.loading));

    final wallets = await _walletRepository.getWallets(
      page: state.page,
      pageSize: 20,
      currency: state.currencyFilter, // Screen-specific filtering
    );

    emit(state.copyWith(
      status: LoadStatus.loaded,
      wallets: [...state.wallets, ...wallets],
      hasMore: wallets.length == 20,
      page: state.page + 1,
    ));
  }

  // Screen-specific: real-time updates
  void onBalanceUpdated(String walletId, double newBalance) {
    final updated = state.wallets.map((w) =>
      w.id == walletId ? w.copyWith(balance: newBalance) : w
    ).toList();
    emit(state.copyWith(wallets: updated));
  }
}

class TransactionHistoryNotifier extends Notifier<TransactionHistoryState> {
  final TransactionRepository _transactionRepository;

  TransactionHistoryNotifier(this._transactionRepository)
      : super(const TransactionHistoryState.initial());

  Future<void> loadTransactions({bool refresh = false}) async {
    if (refresh) {
      emit(state.copyWith(
        status: LoadStatus.loading, page: 0, transactions: [],
      ));
    }

    emit(state.copyWith(status: LoadStatus.loading));

    final transactions = await _transactionRepository.getTransactions(
      page: state.page,
      pageSize: 20,
      from: state.startDate, // Screen-specific filtering
      to: state.endDate,
    );

    emit(state.copyWith(
      status: LoadStatus.loaded,
      transactions: [...state.transactions, ...transactions],
      hasMore: transactions.length == 20,
      page: state.page + 1,
      // Screen-specific: date grouping
      groupedByDate: _groupByDate([...state.transactions, ...transactions]),
    ));
  }

  Map<DateTime, List<Transaction>> _groupByDate(List<Transaction> txns) {
    // Screen-specific grouping logic
    // ...
    return {};
  }
}

// The pagination pattern (page tracking, hasMore, append-on-load) is structural
// similarity — a convention, not shared knowledge. Each screen's pagination
// evolves independently based on its UX requirements.
//
// If pagination infrastructure becomes truly complex (cursor-based pagination,
// optimistic updates, cache invalidation), extract a PaginationHelper that
// manages page state — but let each cubit own its data loading and filtering.
//
// DRY: Inlined wrong abstraction — pagination is a pattern (document it),
// not shared knowledge (don't abstract it)
