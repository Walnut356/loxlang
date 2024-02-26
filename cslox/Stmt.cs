using System.Runtime;
using static Utils;

namespace Stmt;

public interface Any
{
    void Execute(Interpreter interp);

    void Resolve(Resolver res);
}

public record Print(Expr.Any expression) : Any
{
    public void Execute(Interpreter interp)
    {
        Console.WriteLine(LoxString(expression.Evaluate(interp)));
    }

    public void Resolve(Resolver res)
    {
        expression.Resolve(res);
    }
}

public record Expression(Expr.Any expression) : Any
{
    public void Execute(Interpreter interp)
    {
        expression.Evaluate(interp);
    }

    public void Resolve(Resolver res)
    {
        expression.Resolve(res);
    }
}

public record Var(Token ident, Expr.Any? init) : Any
{
    public void Execute(Interpreter interp)
    {
        object? val = init?.Evaluate(interp);

        interp.env.Init(ident.token, val);
    }

    public void Resolve(Resolver res)
    {
        res.Declare(ident);
        init?.Resolve(res);
        res.Define(ident);
    }
}

public record Block(List<Any> stmts) : Any
{
    public void Execute(Interpreter interp)
    {
        Env scope = interp.env;
        try
        {
            interp.env = new Env(interp.env);
            foreach (var stmt in stmts)
            {
                stmt.Execute(interp);
            }
        }
        finally
        {
            interp.env = scope;
        }
    }

    public void Resolve(Resolver res)
    {
        res.Push();
        foreach (var stmt in stmts)
        {
            stmt.Resolve(res);
        }
        res.Pop();
    }
}

public record If(Expr.Any cond, Any if_block, Any? else_block) : Any
{
    public void Execute(Interpreter interp)
    {
        if (Truthy(cond.Evaluate(interp)))
        {
            if_block.Execute(interp);
        }
        else
        {
            else_block?.Execute(interp);
        }
    }

    public void Resolve(Resolver res)
    {
        cond.Resolve(res);
        if_block.Resolve(res);
        else_block?.Resolve(res);
    }
}

public record While(Expr.Any cond, Any block) : Any
{
    public void Execute(Interpreter interp)
    {
        while (Truthy(cond.Evaluate(interp)))
        {
            block.Execute(interp);
        }
    }

    public void Resolve(Resolver res)
    {
        cond.Resolve(res);
        block.Resolve(res);
    }
}

public record Function(Token ident, List<Token> parameters, List<Any> block) : Any
{
    public void Execute(Interpreter interp)
    {
        Callable.Func func = new(this, interp.env);
        interp.env.Init(ident.token, func);
    }

    public void Resolve(Resolver res)
    {
        res.Declare(ident);
        res.Define(ident);

        res.Func(this, FuncType.Function);
    }
}

public record Return(Token ret, Expr.Any? val) : Any
{
    public void Execute(Interpreter interp)
    {
        object? result = val?.Evaluate(interp);

        throw new Abomination(result);
    }

    public void Resolve(Resolver res)
    {
        val?.Resolve(res);
    }
}