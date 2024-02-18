using static TokenType;
using static Utils;

namespace Expr;

/// <summary>
///  Represents any Expression
/// </summary>
public interface Any
{
    // visitor pattern is really gross. I could make this a big switch statement, but it's really
    // not that much easier than just writing out the impl for each function. It's small-scale
    // enough that it's really not that bad.
    public string PrettyPrint();
    public object? Evaluate(Env env);
}

public record Binary(Any left, Token op, Any right) : Any
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

public record Grouping(Any expression) : Any
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

public record Literal(object? value) : Any
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

public record Unary(Token op, Any right) : Any
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

public record Variable(Token ident) : Any
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

public record Assign(Token ident, Any val) : Any
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

public record Logical(Any left, Token op, Any right) : Any
{
    public object? Evaluate(Env env)
    {
        var lhs = left.Evaluate(env);
        // I dislike this just like i dislike the truthy-ness rule. Logical operators should return
        // logical values - i.e. boolean values. The given logic is like adding 2 ints and getting
        // a string. Even if the answer is right ("2"), it's still not the type of value you'd
        // inuitively expect.  But it *can* return boolean values if l or r are already bools.
        // That means a logical operator can return 4 types: Some(object), Null, true, or false. So
        //  now, an expression that usually returns 1 of 2 possibilities essentially returns void*

        // Technically Some(object) could "contain" true and false, but imo they're separate
        // categories. Some/None is the existance/absence of an object. True or false is a value.
        // By equating false and nil, you imply that they have the same semantic meaning - that
        // the *absence* of an object is the same thing as an existing  statement being incorrect
        // which is silly.

        return op.type switch
        {
            OR => Truthy(lhs) ? lhs : right.Evaluate(env),
            AND => Truthy(lhs) ? right.Evaluate(env) : lhs,
        };
    }

    public string PrettyPrint()
    {
        return $"({op.token} {left.PrettyPrint()} {right.PrettyPrint()})";
    }
}
