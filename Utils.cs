static class Utils {
    public static bool IsDigit(char c) {
        return (byte)(c - '0') < 10;
    }

    public static bool IsAlpha(char c) {
        unchecked {
            // why does c# implicitly cast these to u32's?
            return (u8)((u8)(c & 0b1101_1111) - 65) < 26;
        }
    }
}