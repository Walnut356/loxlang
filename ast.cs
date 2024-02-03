
public interface Expr
{
    // visitor pattern is really gross. I could make this a big switch statement, but it's really
    // not that much easier than just writing out the impl for each function. It's small-scale
    // enough that it's really not that bad.
    public string PrettyPrint();
}

public record Binary(Expr left, Token op, Expr right) : Expr
{
    public string PrettyPrint()
    {
        return $"({op.token} {left.PrettyPrint()} {right.PrettyPrint()})";
    }
}

public record Grouping(Expr expression) : Expr
{
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
}

public record Unary(Token op, Expr right) : Expr
{
    public string PrettyPrint()
    {
        return $"({op.token} {right.PrettyPrint()})";
    }
}