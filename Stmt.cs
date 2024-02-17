using static Utils;

public interface Stmt
{
    void Execute(Env env);
}

public record PrintStmt(Expr expression) : Stmt
{
    public void Execute(Env env)
    {
        Console.WriteLine(LoxString(expression.Evaluate(env)));
    }
}

public record ExprStmt(Expr expression) : Stmt
{
    public void Execute(Env env)
    {
        expression.Evaluate(env);
    }
}

public record VarStmt(Token ident, Expr? init) : Stmt
{
    public void Execute(Env env)
    {
        object? val = init?.Evaluate(env);

        env.Init(ident.token, val);
    }
}

public record BlockStmt(List<Stmt> stmts) : Stmt
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