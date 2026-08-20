import { PermissionsAndroid, Platform } from 'react-native';
import { BleManager, type Device, State } from 'react-native-ble-plx';

export const COBOX_SETUP_SERVICE_UUID = 'e9028b60-37f3-4c25-b960-6af1e7150001';
export const COBOX_SETTINGS_CHARACTERISTIC_UUID = 'e9028b60-37f3-4c25-b960-6af1e7150002';
export const COBOX_PROFILE_LENGTH = 106;
export const COBOX_SETUP_OFFSET = 55;
export const COBOX_PERSONALITY_OFFSET = 50;
export const COBOX_BLOB_NAME_OFFSET = 56;
export const COBOX_OWNER_NAME_OFFSET = 72;
export const COBOX_SPEECH_OFFSET = 88;
export const COBOX_ROLL_STATS = 1;
export const COBOX_ROLL_LOOK = 2;

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

    await this.manager.startDeviceScan(null, null, (error, device) => {
      if (error) {
        onError(error.message);
        void this.manager.stopDeviceScan();
      } else if (device && isCobox(device)) {
        onDevice(device);
      }
    });
  }

  stopScan() {
    return this.manager.stopDeviceScan();
  }

  async connect(device: Device) {
    return this.prepareConnection(await device.connect());
  }

  async connectSaved(deviceId: string) {
    return this.prepareConnection(await this.manager.connectToDevice(deviceId));
  }

  private async prepareConnection(connected: Device) {
    const mtuDevice = Platform.OS === 'android' ? await connected.requestMTU(128) : connected;
    const discovered = await mtuDevice.discoverAllServicesAndCharacteristics();

    const profile = await this.readProfileAfterPairing(discovered);

    return { device: discovered, profile };
  }

  private async readProfileAfterPairing(device: Device) {
    let lastError: unknown;
    for (let attempt = 0; attempt < 15; attempt += 1) {
      try {
        return await this.readProfile(device);
      } catch (error) {
        lastError = error;
        await new Promise((resolve) => setTimeout(resolve, 500));
      }
    }
    throw lastError instanceof Error ? lastError : new Error('Cobox pairing timed out.');
  }

  async readProfile(device: Device) {
    const characteristic = await device.readCharacteristicForService(
      COBOX_SETUP_SERVICE_UUID,
      COBOX_SETTINGS_CHARACTERISTIC_UUID
    );
    if (!characteristic.value) {
      throw new Error('Cobox did not return its setup details.');
    }
    const profile = fromBase64(characteristic.value);
    if (profile.length !== COBOX_PROFILE_LENGTH) {
      throw new Error('Cobox returned an unsupported setup profile.');
    }
    return profile;
  }

  writeProfile(device: Device, profile: Uint8Array) {
    if (profile.length !== COBOX_PROFILE_LENGTH) {
      throw new Error('Invalid Cobox setup profile.');
    }
    return device.writeCharacteristicWithResponseForService(
      COBOX_SETUP_SERVICE_UUID,
      COBOX_SETTINGS_CHARACTERISTIC_UUID,
      toBase64(profile)
    );
  }

  roll(device: Device, roll: number) {
    return device.writeCharacteristicWithResponseForService(
      COBOX_SETUP_SERVICE_UUID,
      COBOX_SETTINGS_CHARACTERISTIC_UUID,
      toBase64(new Uint8Array([roll]))
    );
  }

  destroy() {
    return this.manager.destroy();
  }
}

function isCobox(device: Device) {
  return (
    device.name?.toLowerCase() === 'cobox' ||
    device.localName?.toLowerCase() === 'cobox' ||
    device.serviceUUIDs?.some((uuid) => uuid.toLowerCase() === COBOX_SETUP_SERVICE_UUID)
  );
}

function toBase64(bytes: Uint8Array) {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';
  let result = '';
  for (let index = 0; index < bytes.length; index += 3) {
    const value = (bytes[index] << 16) | ((bytes[index + 1] ?? 0) << 8) | (bytes[index + 2] ?? 0);
    result += alphabet[(value >> 18) & 63];
    result += alphabet[(value >> 12) & 63];
    result += index + 1 < bytes.length ? alphabet[(value >> 6) & 63] : '=';
    result += index + 2 < bytes.length ? alphabet[value & 63] : '=';
  }
  return result;
}

function fromBase64(value: string) {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';
  const clean = value.replace(/=+$/, '');
  const bytes: number[] = [];
  let buffer = 0;
  let bits = 0;
  for (const character of clean) {
    const digit = alphabet.indexOf(character);
    if (digit < 0) {
      throw new Error('Cobox returned invalid setup data.');
    }
    buffer = (buffer << 6) | digit;
    bits += 6;
    if (bits >= 8) {
      bits -= 8;
      bytes.push((buffer >> bits) & 255);
    }
  }
  return new Uint8Array(bytes);
}
