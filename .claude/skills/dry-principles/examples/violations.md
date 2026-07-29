# DRY Violations & Anti-Patterns

## 1. Shotgun Surgery — Fee Calculation in Multiple Services

```dart
// BAD — Same fee calculation in two notifiers
class PaymentNotifier extends AsyncNotifier<PaymentState> {
  double _calculateFee(double amount) {
    return amount > 10000 ? amount * 0.01 : amount * 0.015;
  }
}

class TransferNotifier extends AsyncNotifier<TransferState> {
  double _calculateFee(double amount) {
    return amount > 10000 ? amount * 0.01 : amount * 0.015; // Copy
  }
}

// GOOD — Shared FeeCalculator
class FeeCalculator {
  double calculate(double amount) {
    return amount > 10000 ? amount * 0.01 : amount * 0.015;
  }
}
```

## 2. Business Rules in Multiple Places — Validation Drift

```dart
// BAD — Validation in both Flutter form and API call handler
class PaymentFormNotifier extends Notifier<PaymentFormState> {
  String? validateAmount(String? value) {
    final amount = double.tryParse(value ?? '');
    if (amount == null || amount <= 0) return 'Amount must be positive';
    if (amount > 50000) return 'Amount exceeds limit'; // Client-side limit
    return null;
  }
}

// Meanwhile, API returns 422 for amounts > 100000 — different limit!
// User sees form validation pass, then gets server error.

// GOOD — Server is source of truth; client validates for UX, not for rules
class PaymentFormNotifier extends Notifier<PaymentFormState> {
  String? validateAmount(String? value) {
    final amount = double.tryParse(value ?? '');
    if (amount == null || amount <= 0) return 'Amount must be positive';
    // Limit enforced by server — client shows server error if exceeded
    return null;
  }
}
```

## 3. Magic Numbers — Scattered Constants

```dart
// BAD — Magic numbers in Flutter
class AuthNotifier extends AsyncNotifier<AuthState> {
  bool get shouldRefreshToken =>
      state.value!.tokenExpiresAt.difference(DateTime.now()).inSeconds < 300;
}

class ApiClient {
  Duration get tokenTimeout => const Duration(seconds: 3600);
}

// GOOD — Named constants
class AuthConstants {
  static const tokenLifetime = Duration(hours: 1);
  static const refreshThreshold = Duration(minutes: 5);
}
```

## 4. Copy-Paste with Minor Variations

```dart
// BAD — Copy-pasted widget with minor differences
class TripCard extends StatelessWidget {
  final Trip trip;
  const TripCard({super.key, required this.trip});

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(trip.destination, style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 8),
            Text('${trip.distance} km'),
          ],
        ),
      ),
    );
  }
}

class ParkingCard extends StatelessWidget {
  final ParkingSession parking;
  const ParkingCard({super.key, required this.parking});

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(parking.location, style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 8),
            Text('${parking.duration.inMinutes} min'),
          ],
        ),
      ),
    );
  }
}

// These look identical but display different business concepts.
// When TripCard adds a route map preview and ParkingCard adds
// a zone indicator, they'll diverge. Coincidental similarity.
// DRY: Coincidental similarity — trip and parking cards evolve independently
```

## 5. Over-DRY Tests — Shared Base Class Hiding Intent

```dart
// BAD — Over-DRY test setup hiding intent
void main() {
  late MockTripRepository mockTripRepository;
  late MockParkingRepository mockParkingRepository;
  late MockVehicleRepository mockVehicleRepository;
  late MockNotificationService mockNotificationService;
  // 10+ mocks set up for every test, most unused

  setUp(() {
    mockTripRepository = MockTripRepository();
    mockParkingRepository = MockParkingRepository();
    // ... 8 more mocks
    // Which mocks matter for which test? Unclear.
  });

  test('should load trip', () {
    // Uses mockTripRepository, ignores the other 9 mocks
  });
}

// GOOD — Each test sets up what it needs
void main() {
  group('TripOverviewNotifier', () {
    test('loads trips successfully', () async {
      // Only the mocks this test needs — explicit and visible
      final mockApiClient = MockTripApiClient();
      when(() => mockApiClient.getTrips(page: 0, pageSize: 20))
          .thenAnswer((_) async => [testTrip]);

      final container = ProviderContainer(
        overrides: [
          tripApiClientProvider.overrideWithValue(mockApiClient),
        ],
      );

      final notifier = container.read(tripOverviewProvider.notifier);
      await notifier.loadTrips();

      expect(
        container.read(tripOverviewProvider),
        AsyncData([testTrip]),
      );
    });
  });
}
```

## 6. Wrong Abstraction with Boolean Flags

```dart
// BAD — Shared handler with boolean flags for different providers
class RideRequestHandler {
  Future<void> processRequest({
    required RideRequest request,
    required bool isOwnFleet,
    required bool requiresPreAuth,
    required bool validateInsurance,
  }) async {
    if (validateInsurance) {
      await _checkInsurance(request);
    }

    if (requiresPreAuth) {
      if (await _isAlreadyAuthorized(request)) return;
    }

    final vehicleId = isOwnFleet
        ? request.payload['vehicle_id'] as String
        : request.payload['partner_vehicle_ref'] as String;

    // ... rest of processing
  }
}

// GOOD — Separate handlers for separate providers
class OwnFleetRideHandler {
  Future<void> process(RideRequest request) async {
    // Own fleet logic — no flags, no branching
  }
}

class PartnerFleetRideHandler {
  Future<void> process(RideRequest request) async {
    // Partner fleet logic — independent evolution
  }
}
```

## 7. Premature Extraction with 2 Instances

```dart
// BAD — Extracted after seeing only 2 similar widgets
abstract class BaseInfoCard extends StatelessWidget {
  final String title;
  final String subtitle;
  final IconData icon;

  const BaseInfoCard({
    super.key,
    required this.title,
    required this.subtitle,
    required this.icon,
  });
}

class TripInfoCard extends BaseInfoCard { /* ... */ }
class ParkingInfoCard extends BaseInfoCard { /* ... */ }
// Only 2 instances — too early to know if this is a real pattern.
// When TripInfoCard needs a route preview and ParkingInfoCard needs
// a zone map, the base class becomes a constraint.

// GOOD — Keep separate until a third instance confirms the pattern
class TripInfoCard extends StatelessWidget {
  // Self-contained widget — free to evolve independently
}

class ParkingInfoCard extends StatelessWidget {
  // Self-contained widget — free to evolve independently
}

// DRY: 2 instances — too early to extract. Wait for the third.
// See kiss skill, Premature Abstraction anti-pattern.
```

## 8. Frontend/Backend Validation Drift

```dart
// BAD — Client hardcodes business rules that differ from API
class BookingFormValidator {
  static String? validateAmount(String? value) {
    final amount = double.tryParse(value ?? '');
    if (amount == null || amount <= 0) return 'Invalid amount';
    if (amount > 50000) return 'Exceeds limit'; // 50K — but API allows 100K. Drift!
    return null;
  }

  static String? validateVehicleType(String? value) {
    // Missing 'van'! Only allows car and scooter — drift from API!
    if (value != 'car' && value != 'scooter') return 'Invalid vehicle type';
    return null;
  }
}

// GOOD — API is source of truth; client validates format, not business rules
class BookingFormValidator {
  static String? validateAmount(String? value) {
    final amount = double.tryParse(value ?? '');
    if (amount == null || amount <= 0) return 'Invalid amount';
    // Business limit enforced by API — show API error if exceeded
    return null;
  }

  // Vehicle types come from API — no hardcoded list on client
}
// DRY: API contract is single source of truth for business rules
```

## 9. Separate Products Accepting Duplication (GOOD)

```dart
// GOOD — shuttelapp and anwbapp are separate white-label clients

// In clients/shuttelapp/lib/
class TripSummary {
  final String destination;
  final double distance;
  final Duration duration;
  // Subtype-specific fields and behavior
  final LoyaltyPoints loyaltyPoints;
}

// In clients/anwbapp/lib/
class TripSummary {
  final String destination;
  final double distance;
  final Duration duration;
  // ANWB-specific fields and behavior
  final AnwbMemberDiscount memberDiscount;
}

// These look similar but belong to different white-label clients with:
// - Independent release schedules
// - Different business requirements
// - Different evolution paths
//
// Shared models live in core (shuttellibrary/shuttelcore).
// Client-specific models stay in the client package.
// Forcing both into a shared model creates:
// - Lowest-common-denominator design (both constrained)
// - Version synchronization burden
// - Risk of breaking one client when changing the other
//
// DRY: Separate white-label clients — duplication accepted over coupling
```
