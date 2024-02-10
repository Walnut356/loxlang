using static TokenType;
using static Utils;

public interface Expr
{
    // visitor pattern is really gross. I could make this a big switch statement, but it's really
    // not that much easier than just writing out the impl for each function. It's small-scale
    // enough that it's really not that bad.
    public string PrettyPrint();

    public object Evaluate();
}

public record Binary(Expr left, Token op, Expr right) : Expr
{
    public object Evaluate()
    {
        object l = left.Evaluate();
        object r = right.Evaluate();

        return op.type switch
        {
            MINUS => (double)l - (double)r,
            PLUS => (l, r) switch
            {
                (double, double) => (double)l / (double)r,
                (string, string) => $"{(string)l}{(string)r}",
                _ => throw new Exception($"Cannot add string and nonstring values: {l} and {r}")
            },
            FWDSLASH => (double)l / (double)r,
            ASTERISK => (double)l * (double)r,

            GT => (double) l > (double) r,
            GTE => (double)l >= (double) r,
            LT => (double)l < (double) r,
            LTE => (double)l <= (double) r,
            EQ => Equal(l, r),
            NOT_EQ => Equal(l, r),
            _ => throw new Exception($"Invalid operator: {op}"),
        };
    }

    public string PrettyPrint()
    {
        return $"({op.token} {left.PrettyPrint()} {right.PrettyPrint()})";
    }
}

public record Grouping(Expr expression) : Expr
{
    public object Evaluate()
    {
        throw new NotImplementedException();
    }

    public string PrettyPrint()
    {
        return $"(group {expression.PrettyPrint()})";
    }


}

public record Literal(object? value) : Expr
{
    public string PrettyPrint()
    {
        if (value is null) return "nil";
        return value.ToString();
    }

    public object Evaluate()
    {
        return value;
    }
}

public record Unary(Token op, Expr right) : Expr
{
    public string PrettyPrint()
    {
        return $"({op.token} {right.PrettyPrint()})";
    }

    public object Evaluate()
    {
        object r = right.Evaluate();

        switch (op.type)
        {
            case NOT:
                return !Truthy(r);
            case MINUS:
                return -(double)r;

        }

        return null;
    }
}