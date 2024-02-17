using static TokenType;
using static Utils;

public interface Expr
{
    // visitor pattern is really gross. I could make this a big switch statement, but it's really
    // not that much easier than just writing out the impl for each function. It's small-scale
    // enough that it's really not that bad.
    public string PrettyPrint();
    public object? Evaluate(Env env);
}

public record Binary(Expr left, Token op, Expr right) : Expr
{
    public object? Evaluate(Env env)
    {
        object? l = left.Evaluate(env);
        object? r = right.Evaluate(env);

        switch (op.type)
        {
            case MINUS:
                return (double)l - (double)r;
            case PLUS:
                return (l, r) switch
                {
                    (double, double) => (double)l + (double)r,
                    (string, string) => $"{(string)l}{(string)r}",
                    _ => throw new Exception($"Cannot add string and nonstring values: {l} and {r}")
                };
            case FWDSLASH:
                AssertNumber(op, l, r);
                return (double)l / (double)r;
            case ASTERISK:
                AssertNumber(op, l, r);
                return (double)l * (double)r;

            case GT:
                AssertNumber(op, l, r);
                return (double)l > (double)r;
            case GTE:
                AssertNumber(op, l, r);
                return (double)l >= (double)r;
            case LT:
                AssertNumber(op, l, r);
                return (double)l < (double)r;
            case LTE:
                AssertNumber(op, l, r);
                return (double)l <= (double)r;
            case EQ:
                return Equal(l, r);
            case NOT_EQ:
                return Equal(l, r);
            default:
                throw new Exception($"Invalid operator: {op}");
        };
    }

    public string PrettyPrint()
    {
        return $"({op.token} {left.PrettyPrint()} {right.PrettyPrint()})";
    }
}

public record Grouping(Expr expression) : Expr
{
    public object? Evaluate(Env env)
    {
        return expression.Evaluate(env);
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

    public object Evaluate(Env env)
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

    public object? Evaluate(Env env)
    {
        object? r = right.Evaluate(env);

        switch (op.type)
        {
            case NOT:
                return !Truthy(r);
            case MINUS:
                AssertNumber(op, r);
                return -(double)r;

        }

        return null;
    }
}

public record Variable(Token ident) : Expr
{
    public object? Evaluate(Env env)
    {
        return env.Get(ident);
    }

    public string PrettyPrint()
    {
        throw new NotImplementedException();
    }
}

public record Assign(Token ident, Expr val) : Expr
{
    public object? Evaluate(Env env)
    {
        var result = val.Evaluate(env);
        env.Assign(ident, result);
        return result;
    }

    public string PrettyPrint()
    {
        throw new NotImplementedException();
    }
}