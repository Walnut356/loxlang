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

    public static bool Truthy(object? o)
    {
        if (o is null) return false;
        if (o is bool) return (bool)o;
        // wow I kinda hate this truthy-ness rule, specifically because 0 should never equal true.
        // I'm not a huge fan of other language's either. Implicit casting of strings/arrays gets
        // weird. I don't like rust's either though because implict casting 0 and <not 0>
        // into false and true respectively is pretty reasonable
        return true;
    }

    public static bool Equal(object lhs, object rhs) {
        return (lhs, rhs) switch
        {
            (null, null) => true,
            (null, _) => false,
            (_, null) => false,
            (var l, var r) => l == r,
        };
    }

    public static void AssertNumber(Token op, object operand) {
        if (operand is double) { return; }
        throw new RuntimeError(op, "Operand must be a number.");
    }

    public static void AssertNumber(Token op, object lhs, object rhs)
    {
        if (lhs is double && rhs is double) { return; }
        throw new RuntimeError(op, "Operands must be numbers.");
    }

    /// <summary>
    /// Returns a Lox-formatted string representing the given value
    /// </summary>
    /// <returns></returns>
    public static string LoxString(object? o) {
        if (o is null) return "nil";
        if (o is double) {
            var temp = o.ToString();
            if (temp.EndsWith(".0")) {
                temp = temp.Substring(0, temp.Length - 2);
            }
            return temp;
        }

        return o.ToString()!;
    }
}