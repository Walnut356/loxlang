using static Utils;

namespace Stmt;

public interface Any
{
    void Execute(Env env);
}

public record Print(Expr.Any expression) : Any
{
    public void Execute(Env env)
    {
        Console.WriteLine(LoxString(expression.Evaluate(env)));
    }
}

public record Expression(Expr.Any expression) : Any
{
    public void Execute(Env env)
    {
        expression.Evaluate(env);
    }
}

public record Var(Token ident, Expr.Any? init) : Any
{
    public void Execute(Env env)
    {
        object? val = init?.Evaluate(env);

        env.Init(ident.token, val);
    }
}

public record Block(List<Any> stmts) : Any
{
    public void Execute(Env env)
    {
        Env scope = new(env);
        try
        {
            foreach (var stmt in stmts)
            {
                stmt.Execute(scope);
            }
        }
        finally
        {

        }
    }
}

public record If(Expr.Any cond, Any if_block, Any? else_block) : Any
{
    public void Execute(Env env)
    {
        if (Truthy(cond))
        {
            if_block.Execute(env);
        }
        else
        {
            else_block?.Execute(env);
        }
    }
}

public record While(Expr.Any cond, Any block) : Any
{
    public void Execute(Env env)
    {
        while (Truthy(cond.Evaluate(env))) {
            block.Execute(env);
        }
    }
}