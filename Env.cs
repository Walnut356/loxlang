using static Utils;

public record Env()
{
    public Env? parent = null;
    Dictionary<string, object?> vars = [];

    public Env(Env parent)
    {
        this.parent = parent;
        vars = [];
    }

    public void Init(string ident, object? val)
    {
        if (vars.TryGetValue(ident, out var _))
        {
            vars[ident] = val;
        }
        else
        {
            vars.Add(ident, val);
        }
    }

    public object? Get(Token ident)
    {
        object? result;
        if (vars.TryGetValue(ident.token, out result))
        {
            return result;
        }
        Env? temp = parent;
        while (temp is not null)
        {
            if (temp.vars.TryGetValue(ident.token, out result))
            {
                return result;
            }
            temp = temp.parent;
        }

        throw new RuntimeError(ident, "Undefined variable '" + ident.token + "'.");
    }

    public void Assign(Token ident, object? val)
    {
        if (vars.TryGetValue(ident.token, out var _))
        {
            vars[ident.token] = val;
        }
        Env? temp = parent;
        while (temp is not null)
        {
            if (temp.vars.TryGetValue(ident.token, out _))
            {
                temp.vars[ident.token] = val;
            }
            temp = temp.parent;
        }

        throw new RuntimeError(ident, "Undefined variable '" + ident.token + "'.");
    }

    internal void Assign(string token, object? result)
    {
        throw new NotImplementedException();
    }
}