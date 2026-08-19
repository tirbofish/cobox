import { useEffect, useRef, useState } from 'react';
import { FlatList, Pressable, StyleSheet, Text, View } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { CoboxBluetooth, requestBluetoothPermission } from '@/ble';
import type { Device } from 'react-native-ble-plx';

type Status = 'idle' | 'scanning' | 'connecting' | 'connected';

export default function HomeScreen() {
  const bluetoothRef = useRef<CoboxBluetooth | null>(null);
  if (bluetoothRef.current === null) {
    bluetoothRef.current = new CoboxBluetooth();
  }
  const bluetooth = bluetoothRef.current;

  const [devices, setDevices] = useState<Device[]>([]);
  const [status, setStatus] = useState<Status>('idle');
  const [message, setMessage] = useState('Press Back on Cobox, then find it here.');
  const [connectedDevice, setConnectedDevice] = useState<Device | null>(null);

  useEffect(
    () => () => {
      void bluetooth.destroy();
    },
    [bluetooth]
  );

  async function findCobox() {
    const granted = await requestBluetoothPermission();
    if (!granted) {
      setMessage('Bluetooth permission is required to find Cobox.');
      return;
    }

    setDevices([]);
    setConnectedDevice(null);
    setStatus('scanning');
    setMessage('Looking for Cobox...');
    try {
      await bluetooth.scan(
        (device) => {
          setDevices((current) =>
            current.some((candidate) => candidate.id === device.id) ? current : [...current, device]
          );
        },
        (error) => {
          setStatus('idle');
          setMessage(error);
        }
      );
    } catch (error) {
      setStatus('idle');
      setMessage(error instanceof Error ? error.message : 'Bluetooth is unavailable.');
    }
  }

  async function connect(device: Device) {
    setStatus('connecting');
    setMessage('Enter the six-digit code shown on Cobox when your phone asks.');

    try {
      await bluetooth.stopScan();
      const connected = await bluetooth.connect(device);
      setConnectedDevice(connected);
      setStatus('connected');
      setMessage('Cobox is paired and ready.');
    } catch (error) {
      setStatus('idle');
      setMessage(error instanceof Error ? error.message : 'Pairing failed. Press Back and try again.');
    }
  }

  return (
    <SafeAreaView style={styles.screen}>
      <View style={styles.content}>
        <Text style={styles.title}>Cobox</Text>
        <Text style={styles.subtitle}>Pair your device</Text>
        <Text style={styles.instructions}>
          1. Press Back on Cobox.{'\n'}2. Tap Find Cobox.{'\n'}3. Enter the code shown on Cobox.
        </Text>

        <Pressable
          disabled={status === 'scanning' || status === 'connecting'}
          onPress={() => void findCobox()}
          style={({ pressed }) => [
            styles.primaryButton,
            (pressed || status === 'scanning' || status === 'connecting') && styles.buttonPressed,
          ]}>
          <Text style={styles.primaryButtonText}>
            {status === 'scanning' ? 'Finding Cobox...' : 'Find Cobox'}
          </Text>
        </Pressable>

        <Text style={styles.message}>{message}</Text>

        <FlatList
          data={devices}
          keyExtractor={(device) => device.id}
          ListEmptyComponent={
            status === 'scanning' ? <Text style={styles.empty}>No Cobox found yet.</Text> : null
          }
          renderItem={({ item }) => (
            <Pressable
              disabled={status === 'connecting'}
              onPress={() => void connect(item)}
              style={({ pressed }) => [styles.device, pressed && styles.buttonPressed]}>
              <Text style={styles.deviceName}>{item.localName ?? item.name ?? 'Cobox'}</Text>
              <Text style={styles.deviceId}>{item.id}</Text>
            </Pressable>
          )}
          style={styles.devices}
        />

        {connectedDevice && <Text style={styles.connected}>Connected to {connectedDevice.name ?? 'Cobox'}.</Text>}
      </View>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  screen: {
    flex: 1,
    backgroundColor: '#f8fafc',
  },
  content: {
    flex: 1,
    padding: 24,
    gap: 16,
  },
  title: {
    color: '#0f172a',
    fontSize: 32,
    fontWeight: '700',
  },
  subtitle: {
    color: '#334155',
    fontSize: 20,
    fontWeight: '600',
  },
  instructions: {
    color: '#475569',
    fontSize: 16,
    lineHeight: 24,
  },
  primaryButton: {
    alignItems: 'center',
    backgroundColor: '#2563eb',
    borderRadius: 8,
    paddingVertical: 14,
  },
  primaryButtonText: {
    color: '#fff',
    fontSize: 16,
    fontWeight: '600',
  },
  buttonPressed: {
    opacity: 0.65,
  },
  message: {
    color: '#475569',
    fontSize: 14,
    minHeight: 20,
  },
  devices: {
    flexGrow: 0,
  },
  empty: {
    color: '#64748b',
    paddingVertical: 16,
    textAlign: 'center',
  },
  device: {
    backgroundColor: '#fff',
    borderColor: '#cbd5e1',
    borderRadius: 8,
    borderWidth: 1,
    marginBottom: 8,
    padding: 16,
  },
  deviceName: {
    color: '#0f172a',
    fontSize: 16,
    fontWeight: '600',
  },
  deviceId: {
    color: '#64748b',
    fontSize: 12,
    marginTop: 4,
  },
  connected: {
    color: '#166534',
    fontSize: 14,
    fontWeight: '600',
  },
});
