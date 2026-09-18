package Database

import (
	"context"
	"errors"
	"fmt"
	"time"

	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

type Message struct {
	Id        int
	Sender    string
	Content   string
	CreatedAt time.Time
}

type DBService struct {
	Pool *pgxpool.Pool
}

var Instance *DBService

var ErrDBNotInitialized = errors.New("veritabani baglantisi baslatilamadi (pool nil)") //Hata tanimcisi
func ensureDatabaseExists(ctx context.Context) error {

	adminConnstr := "postgres://root:12345@127.0.0.1:5432/postgres?sslmode=disable"
	adminConn, err := pgx.Connect(ctx, adminConnstr)
	if err != nil {
		return fmt.Errorf("Admin Postgres baglantisi kurulamadi: %w", err)
	}

	defer adminConn.Close(ctx)

	var exists bool
	querycheck := "SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = 'nexustui_db');"

	if err := adminConn.QueryRow(ctx, querycheck).Scan(&exists); err != nil {
		return fmt.Errorf("Veritabani Baglantisi Basarisiz: %w", err)
	}

	if !exists {
		fmt.Println("Exists not found, creating that database.......")
		_, err = adminConn.Exec(ctx, "Create Database nexustui_db;")
		if err != nil {
			return fmt.Errorf("veritabani olusturulamadi: %w", err)
		}
		fmt.Println("nexustui Veritanabi just created....")
	}
	return nil
}

func InitDb() error {

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)

	defer cancel()

	if err := ensureDatabaseExists(ctx); err != nil {
		return err
	}

	connStr := "postgres://root:12345@127.0.0.1:5432/nexustui_db?sslmode=disable"

	config, err := pgxpool.ParseConfig(connStr)

	if err != nil {
		return fmt.Errorf("pgxpool config error code:%w", err)
	}

	config.MaxConns = 15                   //Max 15 pool
	config.MinConns = 2                    //en az 2
	config.MaxConnLifetime = 1 * time.Hour // 1 saat

	pool, err := pgxpool.NewWithConfig(ctx, config) //yukaridaki attributeleri ayar yapma....
	if err != nil {
		return fmt.Errorf("pgxpool baglantisi kurulamadi: %w", err)
	}

	if err := pool.Ping(ctx); err != nil {
		return fmt.Errorf("Postgres Ping hatasi : %w", err)
	}

	query := `CREATE TABLE IF NOT EXISTS devices(

	ID SERIAL PRIMARY KEY,
	ip TEXT UNIQUE NOT NULL,
	name TEXT NOT NULL,
	last_seen TIMESTAMP DEFAULT CURRENT_TIMESTAMP
	);


	CREATE TABLE IF NOT EXISTS messages(

	id SERIAL PRIMARY KEY,
	sender TEXT NOT NULL,
	content TEXT NOT NULL,
	created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
	);

	`
	if _, err := pool.Exec(ctx, query); err != nil {
		return fmt.Errorf("Tablolar Olusturulamadi: %w", err)
	}

	Instance = &DBService{Pool: pool}
	fmt.Println("🐘 PostgreSQL (pgx/v5 pgxpool) basariyla baglandi ve tablolar hazir!")
	return nil
}

func SaveDevice(ip, name string) error {
	if Instance == nil || Instance.Pool == nil {
		return ErrDBNotInitialized
	}

	// 1. ctx ve cancel dogru karsilaniyor:
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel() // Artik cancel tanimli!

	query := `INSERT INTO devices (ip, name, last_seen)
		VALUES ($1, $2, CURRENT_TIMESTAMP)
		ON CONFLICT (ip) DO UPDATE SET last_seen = CURRENT_TIMESTAMP, name = $2;`

	// 2. err burada ilk defa error olarak tanimlaniyor:
	_, err := Instance.Pool.Exec(ctx, query, ip, name)
	return err
}

func SaveMessage(sender, content string) error {

	if Instance == nil || Instance.Pool == nil {
		return ErrDBNotInitialized
	}
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	query := `INSERT INTO messages (sender, content) VALUES ($1, $2);`
	_, err := Instance.Pool.Exec(ctx, query, sender, content)
	return err
}

func GetRecentMessages(limit int) ([]Message, error) {

	if Instance == nil || Instance.Pool == nil {
		return nil, ErrDBNotInitialized
	}

	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	query := `SELECT id,sender,content,created_at FROM messages ORDER BY id DESC LIMIT $1;`
	rows, err := Instance.Pool.Query(ctx, query, limit)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var msg []Message
	for rows.Next() {
		var m Message
		if err := rows.Scan(&m.Id, &m.Sender, &m.Content, &m.CreatedAt); err != nil {
			return nil, err
		}

		msg = append(msg, m)
	}

	for i, j := 0, len(msg)-1; i < j; i, j = i+1, j-1 {
		msg[i], msg[j] = msg[j], msg[i]
	}
	return msg, nil
}
