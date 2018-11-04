@0xe76ed23d7b181181;

struct SensorState {
  value @0 :Float32;
}

interface Sensor {
  measure @0 () -> (state :SensorState);
}
