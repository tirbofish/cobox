import { PermissionsAndroid, Platform } from 'react-native';
import { BleManager, type Device, State } from 'react-native-ble-plx';

export const COBOX_SETUP_SERVICE_UUID = 'e9028b60-37f3-4c25-b960-6af1e7150001';
export const COBOX_SETTINGS_CHARACTERISTIC_UUID = 'e9028b60-37f3-4c25-b960-6af1e7150002';

export async function requestBluetoothPermission() {
  if (Platform.OS !== 'android') {
    return true;
  }

  const permissions =
    Number(Platform.Version) >= 31
      ? [
          PermissionsAndroid.PERMISSIONS.BLUETOOTH_SCAN,
          PermissionsAndroid.PERMISSIONS.BLUETOOTH_CONNECT,
        ]
      : [PermissionsAndroid.PERMISSIONS.ACCESS_FINE_LOCATION];
  const results = await Promise.all(
    permissions.map((permission) => PermissionsAndroid.request(permission))
  );

  return results.every((result) => result === PermissionsAndroid.RESULTS.GRANTED);
}

export class CoboxBluetooth {
  private readonly manager = new BleManager();

  async scan(onDevice: (device: Device) => void, onError: (message: string) => void) {
    if ((await this.manager.state()) !== State.PoweredOn) {
      throw new Error('Turn on Bluetooth to find Cobox.');
    }

    await this.manager.startDeviceScan([COBOX_SETUP_SERVICE_UUID], null, (error, device) => {
      if (error) {
        onError(error.message);
        void this.manager.stopDeviceScan();
      } else if (device) {
        onDevice(device);
      }
    });
  }

  stopScan() {
    return this.manager.stopDeviceScan();
  }

  async connect(device: Device) {
    const connected = await device.connect();
    const discovered = await connected.discoverAllServicesAndCharacteristics();

    // Reading this encrypted characteristic makes the OS show its BLE passkey prompt.
    await discovered.readCharacteristicForService(
      COBOX_SETUP_SERVICE_UUID,
      COBOX_SETTINGS_CHARACTERISTIC_UUID
    );

    return discovered;
  }

  destroy() {
    return this.manager.destroy();
  }
}
