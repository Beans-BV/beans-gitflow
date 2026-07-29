# SOLID Violations — BAD/GOOD Examples

## SRP — Single Responsibility Principle

### Violation: TripNotifier does too much

```dart
// BAD — One class handles trip state management AND notification sending
// Two reasons to change: trip logic changes, or notification channel changes
class TripNotifier extends AsyncNotifier<TripState> {
  @override
  Future<TripState> build() async {
    final trip = await ref.read(tripApiProvider).getActiveTrip();
    return TripState(trip: trip);
  }

  Future<void> completeTrip(String tripId) async {
    final api = ref.read(tripApiProvider);
    final trip = await api.completeTrip(tripId);

    // Trip business logic
    state = AsyncData(TripState(trip: trip));

    // Notification logic — completely different responsibility
    final messaging = ref.read(firebaseMessagingProvider);
    await messaging.sendNotification(
      title: 'Trip Completed',
      body: 'Your trip to ${trip.destination} is complete.',
    );

    final dio = ref.read(dioProvider);
    await dio.post(
      'https://hooks.slack.com/...',
      data: {'text': 'Trip ${trip.id} completed'},
    );
  }
}

// GOOD — Separate responsibilities into separate classes
class TripNotifier extends AsyncNotifier<TripState> {
  @override
  Future<TripState> build() async {
    final trip = await ref.read(tripApiProvider).getActiveTrip();
    return TripState(trip: trip);
  }

  Future<void> completeTrip(String tripId) async {
    final api = ref.read(tripApiProvider);
    final trip = await api.completeTrip(tripId);

    state = AsyncData(TripState(trip: trip));

    // Notification is delegated — not this class's responsibility
    await ref.read(tripNotificationServiceProvider).notifyTripCompleted(trip);
  }
}

// Notification is its own responsibility
class TripNotificationService {
  TripNotificationService(this._messaging);

  final FirebaseMessaging _messaging;

  Future<void> notifyTripCompleted(Trip trip) async {
    await _messaging.sendNotification(
      title: 'Trip Completed',
      body: 'Your trip to ${trip.destination} is complete.',
    );
  }
}
```

**Why:** The BAD version changes when trip rules change AND when notification channels change. The GOOD version separates these — `TripNotifier` orchestrates trip state, `TripNotificationService` handles notification delivery.

---

## OCP — Open/Closed Principle

### Violation: Switch statement for vehicle providers

```dart
// BAD — Adding a new provider requires modifying this class
class VehicleLocator {
  Future<VehicleLocation> locate(Vehicle vehicle) async {
    switch (vehicle.provider) {
      case VehicleProvider.internalFleet:
        final client = InternalFleetClient();
        return await client.getLocation(vehicle.id);

      case VehicleProvider.sharedMobility:
        final client = SharedMobilityClient();
        return await client.fetchPosition(vehicle.externalId);

      // Every new provider = modify this class + add a new case
      default:
        throw UnimplementedError(
          'Provider ${vehicle.provider} not supported',
        );
    }
  }
}

// GOOD — New providers are extensions, not modifications
abstract class VehicleLocationAdapter {
  VehicleProvider get provider;
  Future<VehicleLocation> getLocation(Vehicle vehicle);
}

class InternalFleetLocationAdapter implements VehicleLocationAdapter {
  InternalFleetLocationAdapter(this._client);

  final InternalFleetClient _client;

  @override
  VehicleProvider get provider => VehicleProvider.internalFleet;

  @override
  Future<VehicleLocation> getLocation(Vehicle vehicle) =>
      _client.getLocation(vehicle.id);
}

class SharedMobilityLocationAdapter implements VehicleLocationAdapter {
  SharedMobilityLocationAdapter(this._client);

  final SharedMobilityClient _client;

  @override
  VehicleProvider get provider => VehicleProvider.sharedMobility;

  @override
  Future<VehicleLocation> getLocation(Vehicle vehicle) =>
      _client.fetchPosition(vehicle.externalId);
}

// Notifier uses a resolver — adding a new provider = adding a new class
class VehicleLocationNotifier extends AsyncNotifier<VehicleLocation> {
  @override
  Future<VehicleLocation> build() async {
    final vehicle = ref.watch(selectedVehicleProvider);
    final adapters = ref.read(vehicleLocationAdaptersProvider);
    final adapter = adapters.firstWhere(
      (a) => a.provider == vehicle.provider,
    );
    return adapter.getLocation(vehicle);
  }
}
```

**Why:** The BAD version requires modifying `VehicleLocator` every time a new provider is added. The GOOD version is closed for modification — add a new `VehicleLocationAdapter` implementation and register it as a provider.

---

## LSP — Liskov Substitution Principle

### Violation: Data source that doesn't fulfill the contract

```dart
// BAD — ReadOnlyTripDataSource violates the TripDataSource contract
abstract class TripDataSource {
  Future<Trip> getById(String id);
  Future<void> update(Trip trip);
  Future<void> delete(String id);
}

class ReadOnlyTripDataSource implements TripDataSource {
  ReadOnlyTripDataSource(this._dio);

  final Dio _dio;

  @override
  Future<Trip> getById(String id) async {
    final response = await _dio.get('/trips/$id');
    return Trip.fromJson(response.data as Map<String, dynamic>);
  }

  @override
  Future<void> update(Trip trip) =>
      throw UnimplementedError('Read-only data source'); // LSP violation!

  @override
  Future<void> delete(String id) =>
      throw UnsupportedError('Read-only data source'); // LSP violation!
}

// GOOD — Segregate the interface (ISP + LSP working together)
abstract class TripReader {
  Future<Trip> getById(String id);
}

abstract class TripWriter {
  Future<void> update(Trip trip);
  Future<void> delete(String id);
}

// Full data source implements both
class TripDataSource implements TripReader, TripWriter {
  TripDataSource(this._dio);

  final Dio _dio;

  @override
  Future<Trip> getById(String id) async {
    final response = await _dio.get('/trips/$id');
    return Trip.fromJson(response.data as Map<String, dynamic>);
  }

  @override
  Future<void> update(Trip trip) async {
    await _dio.put('/trips/${trip.id}', data: trip.toJson());
  }

  @override
  Future<void> delete(String id) async {
    await _dio.delete('/trips/$id');
  }
}

// Read-only consumers depend only on TripReader — no surprise exceptions
class TripDetailsNotifier extends AsyncNotifier<Trip> {
  @override
  Future<Trip> build() async {
    final tripReader = ref.read(tripReaderProvider); // Only what it needs
    return tripReader.getById(ref.read(selectedTripIdProvider));
  }
}
```

**Why:** The BAD version throws `UnimplementedError` for methods it can't support — any code that calls `update` on what it thinks is a valid `TripDataSource` will crash. The GOOD version splits the interface so read-only consumers never see write methods.

---

## ISP — Interface Segregation Principle

### Violation: Fat parking service interface

```dart
// BAD — One abstract class forces all implementors to support everything
abstract class ParkingService {
  Future<List<ParkingSpot>> searchSpots(LatLng location, double radiusKm);
  Future<ParkingSession> startSession(String spotId, String vehicleId);
  Future<void> stopSession(String sessionId);
  Future<ParkingPayment> processPayment(String sessionId, PaymentMethod method);
  Future<void> sendReceipt(String sessionId, String email);
  Future<List<ParkingSession>> getHistory(String userId);
  Future<void> reportDamage(String spotId, String description);
}

// Search screen only needs spot search — forced to depend on payments, history, etc.
class ParkingSearchNotifier extends AsyncNotifier<List<ParkingSpot>> {
  @override
  Future<List<ParkingSpot>> build() async {
    final service = ref.read(parkingServiceProvider); // Depends on 7 methods, uses 1
    final location = ref.watch(currentLocationProvider);
    return service.searchSpots(location, 5.0);
  }
}

// GOOD — Segregated by client need
abstract class ParkingSpotFinder {
  Future<List<ParkingSpot>> searchSpots(LatLng location, double radiusKm);
}

abstract class ParkingSessionManager {
  Future<ParkingSession> startSession(String spotId, String vehicleId);
  Future<void> stopSession(String sessionId);
}

abstract class ParkingPaymentProcessor {
  Future<ParkingPayment> processPayment(String sessionId, PaymentMethod method);
  Future<void> sendReceipt(String sessionId, String email);
}

abstract class ParkingHistoryReader {
  Future<List<ParkingSession>> getHistory(String userId);
}

// Search screen depends only on what it uses
class ParkingSearchNotifier extends AsyncNotifier<List<ParkingSpot>> {
  @override
  Future<List<ParkingSpot>> build() async {
    final finder = ref.read(parkingSpotFinderProvider);
    final location = ref.watch(currentLocationProvider);
    return finder.searchSpots(location, 5.0);
  }
}
```

**Why:** The BAD version forces every consumer to depend on all parking operations. When payment logic changes, `ParkingSearchNotifier` must be reanalyzed even though it never uses payments. The GOOD version lets each consumer depend only on the operations it actually needs.

---

## DIP — Dependency Inversion Principle

### Violation: Notifier depends on concrete infrastructure

```dart
// BAD — Notifier directly depends on concrete infrastructure classes
class VehicleRegistrationNotifier extends AsyncNotifier<VehicleRegistrationState> {
  @override
  Future<VehicleRegistrationState> build() async {
    return const VehicleRegistrationState.initial();
  }

  Future<void> register(String licensePlate) async {
    // Directly using concrete types — untestable, tightly coupled
    final dio = Dio(BaseOptions(baseUrl: 'https://api.shuttel.nl'));
    final response = await dio.post(
      '/vehicles',
      data: {'license_plate': licensePlate},
    );
    final vehicle = Vehicle.fromJson(response.data as Map<String, dynamic>);

    final prefs = await SharedPreferences.getInstance();
    await prefs.setString('last_vehicle_id', vehicle.id);

    final messaging = FirebaseMessaging.instance;
    await messaging.subscribeToTopic('vehicle_${vehicle.id}');

    state = AsyncData(VehicleRegistrationState.registered(vehicle: vehicle));
  }
}

// GOOD — Depend on abstractions, inject via Riverpod providers
abstract class VehicleApi {
  Future<Vehicle> register(String licensePlate);
}

abstract class VehicleStorage {
  Future<void> saveLastVehicleId(String vehicleId);
}

abstract class VehicleNotificationSubscriber {
  Future<void> subscribeToVehicle(String vehicleId);
}

class VehicleRegistrationNotifier extends AsyncNotifier<VehicleRegistrationState> {
  @override
  Future<VehicleRegistrationState> build() async {
    return const VehicleRegistrationState.initial();
  }

  Future<void> register(String licensePlate) async {
    final api = ref.read(vehicleApiProvider);
    final storage = ref.read(vehicleStorageProvider);
    final subscriber = ref.read(vehicleNotificationSubscriberProvider);

    final vehicle = await api.register(licensePlate);
    await storage.saveLastVehicleId(vehicle.id);
    await subscriber.subscribeToVehicle(vehicle.id);

    state = AsyncData(VehicleRegistrationState.registered(vehicle: vehicle));
  }
}
```

**Why:** The BAD version can't be tested without a real API server, SharedPreferences, and Firebase. The GOOD version depends on abstractions — swap in mocks for testing, swap implementations for different environments.
