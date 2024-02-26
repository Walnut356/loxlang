using static Utils;



public class Resolver
{
    public Interpreter interp;
    public List<Dictionary<string, bool>> scope = new();

    FuncType current_func = FuncType.None;

    public Resolver(Interpreter interp)
    {
        this.interp = interp;
    }

    public void Resolve(List<Stmt.Any> stmts)
    {
        foreach (var stmt in stmts)
        {
            stmt.Resolve(this);
        }
    }

    public void ResolveBlock(List<Stmt.Any> stmts)
    {
        Push();
        foreach (var stmt in stmts)
        {
            stmt.Resolve(this);
        }
        Pop();
    }

    public void Declare(Token ident)
    {
        if (scope.Count == 0)
        {
            return;
        }
        if (scope.Last().ContainsKey(ident.token))
        {
            Lox.Error(ident, "Variable already exists with this name in this scope.");
        }

        scope.Last().Add(ident.token, false);
    }

    public void Define(Token ident)
    {
        if (scope.Count == 0)
        {
            return;
        }

        scope.Last()[ident.token] = true;
    }

    public void Push()
    {
        scope.Add(new());
    }

    public void Pop()
    {
        scope.RemoveAt(scope.Count - 1);
    }

    public void Local(Expr.Any expr, Token ident)
    {
        for (int i = scope.Count - 1; i >= 0; i--)
        {
            if (scope[i].ContainsKey(ident.token))
            {
                interp.Resolve(expr, scope.Count() - 1 - i);
            }
        }
    }

    public void Func(Stmt.Function func, FuncType t)
    {
        var outter = current_func;
        current_func = t;

        Push();
        foreach (Token param in func.parameters)
        {
            Declare(param);
            Define(param);
        }
        Resolve(func.block);
        Pop();

        current_func = outter;
    }
}