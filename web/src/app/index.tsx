import { useEffect, useRef, useState } from 'react';
import { Pressable, ScrollView, StyleSheet, Text, TextInput, View } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';
import AsyncStorage from '@react-native-async-storage/async-storage';

import {
  COBOX_BLOB_NAME_OFFSET,
  COBOX_OWNER_NAME_OFFSET,
  COBOX_ROLL_LOOK,
  COBOX_PERSONALITY_OFFSET,
  COBOX_SETUP_OFFSET,
  COBOX_SPEECH_OFFSET,
  CoboxBluetooth,
  requestBluetoothPermission,
} from '@/ble';
import type { Device } from 'react-native-ble-plx';

type Status = 'idle' | 'scanning' | 'connecting' | 'connected';
const SAVED_COBOX_KEY = 'cobox.device-id';
const SAVED_BLOB_NAME_KEY = 'cobox.blob-name';
const STATS = ['Energy', 'Attention', 'Confidence', 'Playfulness', 'Sleepiness'];

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
  const [profile, setProfile] = useState<Uint8Array | null>(null);
  const [blobName, setBlobName] = useState('');
  const [ownerName, setOwnerName] = useState('');
  const [rolls, setRolls] = useState(0);
  const [savedDeviceId, setSavedDeviceId] = useState<string | null>(null);
  const [savedBlobName, setSavedBlobName] = useState<string | null>(null);

  useEffect(() => {
    void Promise.all([
      AsyncStorage.getItem(SAVED_COBOX_KEY),
      AsyncStorage.getItem(SAVED_BLOB_NAME_KEY),
    ]).then(([deviceId, blobName]) => {
      setSavedDeviceId(deviceId);
      setSavedBlobName(blobName);
    });
    return () => {
      void bluetooth.destroy();
    };
  }, [bluetooth]);

  async function saveDeviceIdentity(device: Device, deviceProfile: Uint8Array) {
    const deviceName = readText(deviceProfile, COBOX_BLOB_NAME_OFFSET, 16);
    await AsyncStorage.multiSet([
      [SAVED_COBOX_KEY, device.id],
      [SAVED_BLOB_NAME_KEY, deviceName],
    ]);
    setSavedDeviceId(device.id);
    setSavedBlobName(deviceName || null);
  }

  async function findCobox() {
    const granted = await requestBluetoothPermission();
    if (!granted) {
      setMessage('Bluetooth permission is required to find Cobox.');
      return;
    }

    setDevices([]);
    setConnectedDevice(null);
    setProfile(null);
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
      setConnectedDevice(connected.device);
      setProfile(connected.profile);
      setBlobName(readText(connected.profile, COBOX_BLOB_NAME_OFFSET, 16));
      setOwnerName(readText(connected.profile, COBOX_OWNER_NAME_OFFSET, 16));
      await saveDeviceIdentity(connected.device, connected.profile);
      setStatus('connected');
      setMessage(connected.profile[COBOX_SETUP_OFFSET] ? 'Cobox is ready.' : 'Make your Cobox yours.');
    } catch (error) {
      setStatus('idle');
      setMessage(error instanceof Error ? error.message : 'Pairing failed. Press Back and try again.');
    }
  }

  async function resumeCobox() {
    if (!savedDeviceId) {
      return;
    }
    const granted = await requestBluetoothPermission();
    if (!granted) {
      setMessage('Bluetooth permission is required to reconnect to Cobox.');
      return;
    }

    setStatus('connecting');
    setMessage('Press Back on Cobox, then reconnecting...');
    try {
      const connected = await bluetooth.connectSaved(savedDeviceId);
      setConnectedDevice(connected.device);
      setProfile(connected.profile);
      setBlobName(readText(connected.profile, COBOX_BLOB_NAME_OFFSET, 16));
      setOwnerName(readText(connected.profile, COBOX_OWNER_NAME_OFFSET, 16));
      await saveDeviceIdentity(connected.device, connected.profile);
      setStatus('connected');
      setMessage(connected.profile[COBOX_SETUP_OFFSET] ? 'Cobox is ready.' : 'Continue setting up your Cobox.');
    } catch (error) {
      setStatus('idle');
      setMessage(error instanceof Error ? error.message : 'Could not reconnect. Press Back on Cobox and try again.');
    }
  }

  async function roll(kind: number) {
    if (!connectedDevice || rolls >= 3) {
      return;
    }
    try {
      await bluetooth.roll(connectedDevice, kind);
      const updated = await bluetooth.readProfile(connectedDevice);
      setProfile(updated);
      setRolls((current) => current + 1);
      setMessage(`${3 - rolls - 1} setup rolls remaining.`);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Cobox could not roll again.');
    }
  }

  async function finishSetup() {
    if (!connectedDevice || !profile) {
      return;
    }
    const next = new Uint8Array(profile);
    if (!writeText(next, COBOX_BLOB_NAME_OFFSET, 16, blobName) || !writeText(next, COBOX_OWNER_NAME_OFFSET, 16, ownerName)) {
      setMessage('Use a Cobox and owner name of up to 16 letters, numbers, or punctuation.');
      return;
    }
    next[COBOX_SETUP_OFFSET] = 1;
    try {
      await bluetooth.writeProfile(connectedDevice, next);
      const savedProfile = await bluetooth.readProfile(connectedDevice);
      const savedName = readText(savedProfile, COBOX_BLOB_NAME_OFFSET, 16);
      if (savedProfile[COBOX_SETUP_OFFSET] !== 1 || savedName !== blobName) {
        throw new Error('Cobox did not save setup. Try again.');
      }
      setProfile(savedProfile);
      await saveDeviceIdentity(connectedDevice, savedProfile);
      setMessage('Cobox is set up and saved.');
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Cobox could not save setup.');
    }
  }

  async function setStat(index: number, value: number) {
    if (!connectedDevice || !profile) {
      return;
    }
    const next = new Uint8Array(profile);
    next[COBOX_PERSONALITY_OFFSET + index] = Math.max(0, Math.min(100, value));
    try {
      await bluetooth.writeProfile(connectedDevice, next);
      setProfile(next);
      setMessage(`${STATS[index]} updated on Cobox.`);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Cobox could not update its stats.');
    }
  }

  async function setSpeech(speech: string) {
    if (!connectedDevice || !profile) {
      return;
    }
    const next = new Uint8Array(profile);
    if (!writeText(next, COBOX_SPEECH_OFFSET, 18, speech)) {
      return;
    }
    try {
      await bluetooth.writeProfile(connectedDevice, next);
      setProfile(next);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Cobox could not update its speech.');
    }
  }

  const needsSetup = profile?.[COBOX_SETUP_OFFSET] !== 1;

  return (
    <SafeAreaView style={styles.screen}>
      <ScrollView contentContainerStyle={styles.content}>
        <Text style={styles.title}>Cobox</Text>
        {!connectedDevice ? (
          <>
            <Text style={styles.subtitle}>{savedDeviceId ? 'Your saved Cobox' : 'Pair your device'}</Text>
            <Text style={styles.instructions}>
              {savedDeviceId
                ? 'Press Back on Cobox, then reconnect to finish setup.'
                : '1. Press Back on Cobox.\n2. Tap Find Cobox.\n3. Enter the code shown on Cobox.'}
            </Text>

            {savedDeviceId ? (
              <Pressable
                disabled={status === 'connecting'}
                onPress={() => void resumeCobox()}
                style={({ pressed }) => [
                  styles.primaryButton,
                  (pressed || status === 'connecting') && styles.buttonPressed,
                ]}>
                {status === 'connecting' ? (
                  <Text style={styles.primaryButtonText}>Connecting...</Text>
                ) : (
                  <Text style={styles.primaryButtonText}>
                    Say hello to <Text style={styles.italic}>{savedBlobName ?? 'your Cobox'}</Text>
                  </Text>
                )}
              </Pressable>
            ) : null}
            <Pressable
              disabled={status === 'scanning' || status === 'connecting'}
              onPress={() => void findCobox()}
              style={({ pressed }) => [
                styles.primaryButton,
                (pressed || status === 'scanning' || status === 'connecting') && styles.buttonPressed,
              ]}>
              <Text style={styles.primaryButtonText}>{status === 'scanning' ? 'Finding Cobox...' : 'Pair a Cobox'}</Text>
            </Pressable>

            {devices.map((device) => (
              <Pressable
                disabled={status === 'connecting'}
                key={device.id}
                onPress={() => void connect(device)}
                style={({ pressed }) => [styles.device, pressed && styles.buttonPressed]}>
                <Text style={styles.deviceName}>{device.localName ?? device.name ?? 'Cobox'}</Text>
                <Text style={styles.deviceId}>{device.id}</Text>
              </Pressable>
            ))}
            {status === 'scanning' && devices.length === 0 ? <Text style={styles.empty}>No Cobox found yet.</Text> : null}
          </>
        ) : needsSetup ? (
          <View style={styles.setup}>
            <Text style={styles.subtitle}>Set up your blob</Text>
            <Text style={styles.saved}>Cobox saved on this phone</Text>
            <Text style={styles.preview}>{blobName || 'Blob'}</Text>
            <Text style={styles.setupState}>Setup in progress</Text>
            <Text style={styles.namePrompt}>My name is</Text>
            <TextInput
              value={blobName}
              onChangeText={setBlobName}
              maxLength={16}
              placeholder="your blob's name"
              style={styles.input}
            />
            <Text style={styles.namePrompt}>and you are</Text>
            <TextInput
              value={ownerName}
              onChangeText={setOwnerName}
              maxLength={16}
              placeholder="your name"
              style={styles.input}
            />
            <Text style={styles.rolls}>{rolls}/3 appearance changes used</Text>
            <View style={styles.row}>
              <Pressable disabled={rolls >= 3} onPress={() => void roll(COBOX_ROLL_LOOK)} style={styles.secondaryButton}>
                <Text style={styles.secondaryButtonText}>Roll look</Text>
              </Pressable>
            </View>
            <Text style={styles.namePrompt}>Stats</Text>
            {STATS.map((stat, index) => {
              const value = profile?.[COBOX_PERSONALITY_OFFSET + index] ?? 0;
              return (
                <View key={stat} style={styles.stat}>
                  <Text style={styles.statName}>{stat}</Text>
                  <View style={styles.row}>
                    <Pressable
                      disabled={value === 0}
                      onPress={() => void setStat(index, value - 1)}
                      style={styles.secondaryButton}>
                      <Text style={styles.secondaryButtonText}>-</Text>
                    </Pressable>
                    <Text style={styles.statValue}>{value}</Text>
                    <Pressable
                      disabled={value === 100}
                      onPress={() => void setStat(index, value + 1)}
                      style={styles.secondaryButton}>
                      <Text style={styles.secondaryButtonText}>+</Text>
                    </Pressable>
                  </View>
                </View>
              );
            })}
            <Pressable onPress={() => void finishSetup()} style={styles.primaryButton}>
              <Text style={styles.primaryButtonText}>Setup my Cobox</Text>
            </Pressable>
          </View>
        ) : (
          <View style={styles.setup}>
            <Text style={styles.subtitle}>{blobName}</Text>
            <Text style={styles.instructions}>Owned by {ownerName}</Text>
            <Text style={styles.rolls}>Make your blob speak</Text>
            <View style={styles.row}>
              {['Hello!', 'Yay!', 'Zzz...', 'Love you!'].map((speech) => (
                <Pressable key={speech} onPress={() => void setSpeech(speech)} style={styles.secondaryButton}>
                  <Text style={styles.secondaryButtonText}>{speech}</Text>
                </Pressable>
              ))}
            </View>
          </View>
        )}

        <Text style={styles.message}>{message}</Text>
      </ScrollView>
    </SafeAreaView>
  );
}

function readText(profile: Uint8Array, offset: number, length: number) {
  const bytes = profile.slice(offset, offset + length);
  const end = bytes.indexOf(0);
  return String.fromCharCode(...bytes.slice(0, end < 0 ? length : end));
}

function writeText(profile: Uint8Array, offset: number, length: number, text: string) {
  if (!/^[\x20-\x7e]{1,}$/.test(text)) {
    return false;
  }
  const bytes = Array.from(text, (character) => character.charCodeAt(0));
  if (bytes.length > length) {
    return false;
  }
  profile.fill(0, offset, offset + length);
  profile.set(bytes, offset);
  return true;
}

const styles = StyleSheet.create({
  screen: {
    flex: 1,
    backgroundColor: '#f8fafc',
  },
  content: {
    flexGrow: 1,
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
  italic: {
    fontStyle: 'italic',
  },
  buttonPressed: {
    opacity: 0.65,
  },
  message: {
    color: '#475569',
    fontSize: 14,
    minHeight: 20,
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
  setup: {
    gap: 12,
  },
  input: {
    backgroundColor: '#fff',
    borderColor: '#cbd5e1',
    borderRadius: 8,
    borderWidth: 1,
    fontSize: 16,
    padding: 14,
  },
  row: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 8,
  },
  secondaryButton: {
    backgroundColor: '#dbeafe',
    borderRadius: 8,
    paddingHorizontal: 12,
    paddingVertical: 10,
  },
  secondaryButtonText: {
    color: '#1e3a8a',
    fontWeight: '600',
  },
  rolls: {
    color: '#475569',
    fontSize: 14,
  },
  saved: {
    color: '#166534',
    fontSize: 14,
    fontWeight: '600',
  },
  setupState: {
    alignSelf: 'flex-start',
    backgroundColor: '#fef3c7',
    borderRadius: 999,
    color: '#92400e',
    fontSize: 13,
    fontWeight: '700',
    overflow: 'hidden',
    paddingHorizontal: 10,
    paddingVertical: 6,
  },
  namePrompt: {
    color: '#334155',
    fontSize: 16,
    fontWeight: '600',
  },
  stat: {
    alignItems: 'center',
    flexDirection: 'row',
    justifyContent: 'space-between',
  },
  statName: {
    color: '#334155',
    fontSize: 16,
    fontWeight: '600',
  },
  statValue: {
    color: '#0f172a',
    fontSize: 18,
    fontVariant: ['tabular-nums'],
    minWidth: 36,
    textAlign: 'center',
  },
  preview: {
    alignSelf: 'center',
    backgroundColor: '#a7f3d0',
    borderRadius: 48,
    color: '#14532d',
    fontSize: 20,
    fontWeight: '700',
    overflow: 'hidden',
    paddingHorizontal: 24,
    paddingVertical: 28,
  },
});
