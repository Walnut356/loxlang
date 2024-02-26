using System.Net;
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
    public object? Evaluate(Interpreter interp);
    public void Resolve(Resolver res);
}

public record Binary(Any left, Token op, Any right) : Any
{
    public object? Evaluate(Interpreter interp)
    {
        object? l = left.Evaluate(interp);
        object? r = right.Evaluate(interp);

        switch (op.type)
        {
            case MINUS:
                return (double)l - (double)r;
            case PLUS:
                return (l, r) switch
                {
                    (double, double) => (double)l + (double)r,
                    (string, string) => $"{(string)l}{(string)r}",
                    (string, double) => $"{(string)l}{(double)r}",
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

    public void Resolve(Resolver res)
    {
        left.Resolve(res);
        right.Resolve(res);
    }
}

public record Grouping(Any expression) : Any
{
    public object? Evaluate(Interpreter interp)
    {
        return expression.Evaluate(interp);
    }

    public string PrettyPrint()
    {
        return $"(group {expression.PrettyPrint()})";
    }

    public void Resolve(Resolver res)
    {
        expression.Resolve(res);
    }
}

public record Literal(object? value) : Any
{
    public string PrettyPrint()
    {
        if (value is null) return "nil";
        return value.ToString();
    }

    public object? Evaluate(Interpreter _interp)
    {
        return value;
    }

    public void Resolve(Resolver res)
    {
    }
}

public record Unary(Token op, Any right) : Any
{
    public string PrettyPrint()
    {
        return $"({op.token} {right.PrettyPrint()})";
    }

    public object? Evaluate(Interpreter interp)
    {
        object? r = right.Evaluate(interp);

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

    public void Resolve(Resolver res)
    {
        right.Resolve(res);
    }
}

public record Variable(Token ident) : Any
{
    public object? Evaluate(Interpreter interp)
    {
        return interp.locals.TryGetValue(this, out int dist) ?
            interp.env.GetAt(dist, ident) : interp.globals.Get(ident);
    }

    public string PrettyPrint()
    {
        return ident.token;
    }

    public void Resolve(Resolver res)
    {
        if (!(res.scope.Count == 0) && res.scope.Last().ContainsKey(ident.token) && res.scope.Last()[ident.token] == false) {
            Lox.Error(ident, "Cannot read local variable in its initializer");
        }

        res.Local(this, ident);
    }
}

public record Assign(Token ident, Any val) : Any
{
    public object? Evaluate(Interpreter interp)
    {
        var result = val.Evaluate(interp);

        if (interp.locals.TryGetValue(this, out int dist)) {
            interp.env.AssignAt(dist, ident, result);
        } else {
            interp.env.Assign(ident, result);
        }

        return result;
    }

    public string PrettyPrint()
    {
        throw new NotImplementedException();
    }

    public void Resolve(Resolver res)
    {
        val.Resolve(res);
        res.Local(this, ident);
    }
}

public record Logical(Any left, Token op, Any right) : Any
{
    public object? Evaluate(Interpreter interp)
    {
        var lhs = left.Evaluate(interp);
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
            OR => Truthy(lhs) ? lhs : right.Evaluate(interp),
            AND => Truthy(lhs) ? right.Evaluate(interp) : lhs,
            _ => throw new Exception($"Unexpected operator '{op.token}' in logical expression"),
        };
    }

    public string PrettyPrint()
    {
        return $"({op.token} {left.PrettyPrint()} {right.PrettyPrint()})";
    }

    public void Resolve(Resolver res)
    {
        left.Resolve(res);
        right.Resolve(res);
    }
}

public record Call(Any func, Token delim, List<Any> args) : Any
{
    public object? Evaluate(Interpreter interp)
    {
        Callable.Any resolved;
        try
        {
            resolved = (Callable.Any)func.Evaluate(interp);
        }
        catch
        {
            throw new RuntimeError(delim, "Resolved object is not callable.");
        }
        if (args.Count != resolved?.Arity())
        {
            throw new RuntimeError(delim, $"Expected {resolved.Arity()} args, got {args.Count}.");
        }
        List<object?> res_args = args.ConvertAll(arg => arg.Evaluate(interp));

        return resolved.Call(interp, res_args);
    }

    public string PrettyPrint()
    {
        return $"{func}({args})";
    }

    public void Resolve(Resolver res)
    {
        func.Resolve(res);

        foreach(var arg in args) {
            arg.Resolve(res);
        }
    }
}