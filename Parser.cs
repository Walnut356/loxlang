using static TokenType;

class Parser
{
    List<Token> tokens;
    int i = 0;

    public Parser(List<Token> tokens)
    {
        this.tokens = tokens;
    }

    public List<Stmt.Any> Parse()
    {
        List<Stmt.Any> stmts = [];
        while (!EOF())
        {
            stmts.Add(Declaration());
        }

        return stmts;
    }

    Expr.Any Expression()
    {
        return Assignment();
    }

    Expr.Any Assignment()
    {
        Expr.Any expr = Or();

        if (tokens[i].type is EQ)
        {
            Token location = tokens[i];
            i++;
            Expr.Any val = Assignment();

            if (expr is Expr.Variable variable)
            {
                Token ident = variable.ident;
                return new Expr.Assign(ident, val);
            }

            Lox.Error(location, "Invalid assignment target.");
        }

        return expr;
    }

    Expr.Any Or() {
        Expr.Any expr = And();

        while (tokens[i].type == OR) {
            Token op = tokens[i];
            i++;

            Expr.Any right = And();
            expr = new Expr.Logical(expr, op, right);
        }

        return expr;
    }

    Expr.Any And() {
        Expr.Any expr = Equality();

        while (tokens[i].type == AND)
        {
            Token op = tokens[i];
            i++;

            Expr.Any right = Equality();
            expr = new Expr.Logical(expr, op, right);
        }

        return expr;
    }

    Expr.Any Equality()
    {
        Expr.Any expression = Comparison();

        while (tokens[i].type is NOT_EQ or DBL_EQ)
        {
            Token op = tokens[i];
            ++i;
            Expr.Any right = Comparison();
            expression = new Expr.Binary(expression, op, right);
        }

        return expression!;
    }

    Expr.Any Comparison()
    {
        Expr.Any expression = Term();

        while (tokens[i].type is GT or LT or GTE or LTE)
        {
            Token op = tokens[i];
            ++i;
            Expr.Any right = Term();
            expression = new Expr.Binary(expression, op, right);
        }

        return expression!;
    }

    Expr.Any Term()
    {
        Expr.Any expression = Factor();

        while (tokens[i].type is MINUS or PLUS)
        {
            Token op = tokens[i];
            ++i;
            Expr.Any right = Factor();
            expression = new Expr.Binary(expression, op, right);
        }

        return expression!;
    }

    Expr.Any Factor()
    {
        Expr.Any expression = Unary();

        while (tokens[i].type is ASTERISK or FWDSLASH)
        {
            Token op = tokens[i];
            ++i;
            Expr.Any right = Unary();
            expression = new Expr.Binary(expression, op, right);
        }

        return expression!;
    }

    Expr.Any Unary()
    {
        if (tokens[i].type is NOT or MINUS)
        {
            Token op = tokens[i];
            i++;
            Expr.Any right = Unary();
            return new Expr.Unary(op, right);
        }

        return Primary();
    }

    Expr.Any Primary()
    {
        var token = tokens[i];
        i++;
        switch (token.type)
        {
            case FALSE:
                return new Expr.Literal(false);
            case TRUE:
                return new Expr.Literal(true);
            case NIL:
                return new Expr.Literal(null);
            case NUMBER:
                return new Expr.Literal(token.literal);
            case STRING:
                return new Expr.Literal(token.literal);
            case PAREN_O:
                Expr.Any expr = Expression();
                Expect(PAREN_C, "Expected ')' after expression.");
                return new Expr.Grouping(expr);
            case IDENTIFIER:
                return new Expr.Variable(token);
            default:
                throw Error(token, "Expected expression");
        }
    }

    bool EOF()
    {
        return i >= tokens.Count || tokens[i].type == TokenType.EOF;
    }

    void Expect(TokenType t, string message)
    {
        if (tokens[i].type != t) { throw Error(tokens[i], message); }
        ++i;
    }

    class ParseError : Exception { }

    ParseError Error(Token token, string message)
    {
        Lox.Error(token, message);
        return new ParseError();
    }

    void Sync()
    {
        while (!EOF())
        {
            if (tokens[i - 1].type == SEMICOLON) return;
            switch (tokens[i + 1].type)
            {
                case CLASS:
                case FUN:
                case VAR:
                case FOR:
                case IF:
                case WHILE:
                case PRINT:
                case RETURN:
                    return;
            }
            i++;
        }
    }

    Stmt.Any Statement()
    {
        var token = tokens[i];
        return token.type switch
        {
            IF => IfStmt(),
            PRINT => PrintStmt(),
            BRACKET_O => new Stmt.Block(Block()),
            _ => ExprStmt(),
        };
    }

    Stmt.Any Declaration()
    {
        try
        {
            var token = tokens[i];
            return token.type switch
            {
                VAR => VarDecl(),
                _ => Statement(),
            };
        }
        catch (ParseError e)
        {
            Sync();
            return null;
        }
    }

    Stmt.Any PrintStmt()
    {
        i++;
        Expr.Any val = Expression();
        Expect(SEMICOLON, "Expected semicolon after value of print statement");
        return new Stmt.Print(val);
    }

    Stmt.Any ExprStmt()
    {
        Expr.Any val = Expression();
        Expect(SEMICOLON, "Expected semicolon after expression");
        return new Stmt.Expression(val);
    }

    Stmt.Any VarDecl()
    {
        i++;
        Expect(IDENTIFIER, "Expected a variable name.");
        Token ident = tokens[i - 1];

        Expr.Any init = null;

        if (tokens[i].type == EQ)
        {
            i++;
            init = Expression();
        }

        Expect(SEMICOLON, "Expected semicolon after var declaration");
        return new Stmt.Var(ident, init);
    }

    List<Stmt.Any> Block()
    {
        List<Stmt.Any> stmts = [];
        i++;

        while (!(tokens[i].type == BRACKET_C) && !EOF())
        {
            stmts.Add(Declaration());
        }

        Expect(BRACKET_C, "Expected '}' to close block");
        return stmts;
    }

    Stmt.Any IfStmt()
    {
        Expect(PAREN_O, "Expected '(' after 'if'.");
        Expr.Any cond = Expression();
        Expect(PAREN_C, "Expected ')' after 'if' condition");

        Stmt.Any if_block = Statement();
        i++;
        Stmt.Any? else_block = tokens[i - 1].type is ELSE ? Statement() : null;

        return new Stmt.If(cond, if_block, else_block);
    }
}