# Cobox mobile app

The app uses Bluetooth Low Energy and therefore runs in an Expo development build, not Expo Go.

1. Install the Android development build:

   ```bash
   bun run android
   ```

   For iOS, use `bun run ios`.

2. Start Metro for the installed development build:

   ```bash
   bun run start
   ```

3. Press **Back** on Cobox to open its pairing window, then use **Find Cobox** in the app and enter the six-digit code shown on the device when your phone asks.
