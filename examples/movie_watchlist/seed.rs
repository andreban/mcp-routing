// Copyright 2026 André Cipriani Bandarra
// SPDX-License-Identifier: Apache-2.0

//! Pre-seeded catalog data and multi-tenant demo user profiles for CineList.

use std::collections::HashMap;

use super::models::{Movie, MovieDb, MovieRating, UserProfile, Watchlist, WatchlistItem};

/// Returns a pre-populated [`MovieDb`] containing curated movies and test user accounts.
pub fn seed_database() -> MovieDb {
    let mut catalog = HashMap::new();

    let movies = vec![
        Movie {
            id: "interstellar".to_string(),
            title: "Interstellar".to_string(),
            year: 2014,
            director: "Christopher Nolan".to_string(),
            cast: vec!["Matthew McConaughey".to_string(), "Anne Hathaway".to_string(), "Jessica Chastain".to_string()],
            genres: vec!["Sci-Fi".to_string(), "Drama".to_string()],
            runtime_minutes: 169,
            rating: 8.7,
            synopsis: "A team of explorers travel through a wormhole in space in an attempt to ensure humanity's survival.".to_string(),
            streaming_platforms: vec!["Netflix".to_string(), "Prime Video".to_string()],
        },
        Movie {
            id: "blade-runner-2049".to_string(),
            title: "Blade Runner 2049".to_string(),
            year: 2017,
            director: "Denis Villeneuve".to_string(),
            cast: vec!["Ryan Gosling".to_string(), "Harrison Ford".to_string(), "Ana de Armas".to_string()],
            genres: vec!["Sci-Fi".to_string(), "Thriller".to_string()],
            runtime_minutes: 164,
            rating: 8.0,
            synopsis: "Young Blade Runner K's discovery of a long-buried secret leads him to track down former Blade Runner Rick Deckard.".to_string(),
            streaming_platforms: vec!["Max".to_string()],
        },
        Movie {
            id: "2001-a-space-odyssey".to_string(),
            title: "2001: A Space Odyssey".to_string(),
            year: 1968,
            director: "Stanley Kubrick".to_string(),
            cast: vec!["Keir Dullea".to_string(), "Gary Lockwood".to_string()],
            genres: vec!["Sci-Fi".to_string()],
            runtime_minutes: 149,
            rating: 8.3,
            synopsis: "After uncovering a mysterious artifact on the Moon, a spacecraft is sent to Jupiter to find its origins.".to_string(),
            streaming_platforms: vec!["Criterion Channel".to_string(), "Max".to_string()],
        },
        Movie {
            id: "arrival".to_string(),
            title: "Arrival".to_string(),
            year: 2016,
            director: "Denis Villeneuve".to_string(),
            cast: vec!["Amy Adams".to_string(), "Jeremy Renner".to_string(), "Forest Whitaker".to_string()],
            genres: vec!["Sci-Fi".to_string(), "Drama".to_string()],
            runtime_minutes: 116,
            rating: 7.9,
            synopsis: "A linguist works with the military to communicate with alien lifeforms after mysterious spacecraft land around the world.".to_string(),
            streaming_platforms: vec!["Netflix".to_string()],
        },
        Movie {
            id: "the-dark-knight".to_string(),
            title: "The Dark Knight".to_string(),
            year: 2008,
            director: "Christopher Nolan".to_string(),
            cast: vec!["Christian Bale".to_string(), "Heath Ledger".to_string(), "Aaron Eckhart".to_string()],
            genres: vec!["Action".to_string(), "Crime".to_string(), "Drama".to_string()],
            runtime_minutes: 152,
            rating: 9.0,
            synopsis: "When the menace known as the Joker wreaks havoc and chaos on the people of Gotham, Batman must accept one of the greatest tests.".to_string(),
            streaming_platforms: vec!["Max".to_string()],
        },
        Movie {
            id: "parasite".to_string(),
            title: "Parasite".to_string(),
            year: 2019,
            director: "Bong Joon-ho".to_string(),
            cast: vec!["Song Kang-ho".to_string(), "Lee Sun-kyun".to_string(), "Cho Yeo-jeong".to_string()],
            genres: vec!["Thriller".to_string(), "Drama".to_string(), "Comedy".to_string()],
            runtime_minutes: 132,
            rating: 8.5,
            synopsis: "Greed and class discrimination threaten the newly formed symbiotic relationship between the wealthy Park and destitute Kim families.".to_string(),
            streaming_platforms: vec!["Max".to_string(), "Criterion Channel".to_string()],
        },
        Movie {
            id: "spirited-away".to_string(),
            title: "Spirited Away".to_string(),
            year: 2001,
            director: "Hayao Miyazaki".to_string(),
            cast: vec!["Rumi Hiiragi".to_string(), "Miyu Irino".to_string()],
            genres: vec!["Animation".to_string(), "Adventure".to_string()],
            runtime_minutes: 125,
            rating: 8.6,
            synopsis: "During her family's move to the suburbs, a 10-year-old girl wanders into a world ruled by gods, witches, and spirits.".to_string(),
            streaming_platforms: vec!["Max".to_string()],
        },
        Movie {
            id: "se7en".to_string(),
            title: "Se7en".to_string(),
            year: 1995,
            director: "David Fincher".to_string(),
            cast: vec!["Brad Pitt".to_string(), "Morgan Freeman".to_string(), "Gwyneth Paltrow".to_string()],
            genres: vec!["Thriller".to_string(), "Crime".to_string(), "Drama".to_string()],
            runtime_minutes: 127,
            rating: 8.6,
            synopsis: "Two detectives hunt a serial killer who uses the seven deadly sins as his motives.".to_string(),
            streaming_platforms: vec!["Max".to_string()],
        },
        Movie {
            id: "the-matrix".to_string(),
            title: "The Matrix".to_string(),
            year: 1999,
            director: "The Wachowskis".to_string(),
            cast: vec!["Keanu Reeves".to_string(), "Laurence Fishburne".to_string(), "Carrie-Anne Moss".to_string()],
            genres: vec!["Sci-Fi".to_string(), "Action".to_string()],
            runtime_minutes: 136,
            rating: 8.7,
            synopsis: "A computer hacker learns from mysterious rebels about the true nature of his reality and his role in the war against its controllers.".to_string(),
            streaming_platforms: vec!["Prime Video".to_string()],
        },
        Movie {
            id: "whiplash".to_string(),
            title: "Whiplash".to_string(),
            year: 2014,
            director: "Damien Chazelle".to_string(),
            cast: vec!["Miles Teller".to_string(), "J.K. Simmons".to_string()],
            genres: vec!["Drama".to_string(), "Music".to_string()],
            runtime_minutes: 106,
            rating: 8.5,
            synopsis: "A promising young drummer enrolls at a cut-throat music conservatory where his dreams of greatness are mentored by an instructor who will stop at nothing.".to_string(),
            streaming_platforms: vec!["Netflix".to_string()],
        },
        Movie {
            id: "alien".to_string(),
            title: "Alien".to_string(),
            year: 1979,
            director: "Ridley Scott".to_string(),
            cast: vec!["Sigourney Weaver".to_string(), "Tom Skerritt".to_string()],
            genres: vec!["Sci-Fi".to_string(), "Horror".to_string()],
            runtime_minutes: 117,
            rating: 8.5,
            synopsis: "The crew of a commercial spacecraft encounters a deadly lifeform after investigating a mysterious transmission.".to_string(),
            streaming_platforms: vec!["Prime Video".to_string()],
        },
        Movie {
            id: "the-grand-budapest-hotel".to_string(),
            title: "The Grand Budapest Hotel".to_string(),
            year: 2014,
            director: "Wes Anderson".to_string(),
            cast: vec!["Ralph Fiennes".to_string(), "F. Murray Abraham".to_string()],
            genres: vec!["Comedy".to_string(), "Drama".to_string()],
            runtime_minutes: 99,
            rating: 8.1,
            synopsis: "A writer encounters the owner of an aging high-class hotel, who tells him of his early years serving as a lobby boy in the hotel's glorious years.".to_string(),
            streaming_platforms: vec!["Max".to_string()],
        },
        Movie {
            id: "pulp-fiction".to_string(),
            title: "Pulp Fiction".to_string(),
            year: 1994,
            director: "Quentin Tarantino".to_string(),
            cast: vec!["John Travolta".to_string(), "Samuel L. Jackson".to_string(), "Uma Thurman".to_string()],
            genres: vec!["Crime".to_string(), "Drama".to_string()],
            runtime_minutes: 154,
            rating: 8.9,
            synopsis: "The lives of two mob hitmen, a boxer, a gangster and his wife intertwine in four tales of violence and redemption.".to_string(),
            streaming_platforms: vec!["Netflix".to_string()],
        },
        Movie {
            id: "spider-man-into-the-spider-verse".to_string(),
            title: "Spider-Man: Into the Spider-Verse".to_string(),
            year: 2018,
            director: "Bob Persichetti, Peter Ramsey".to_string(),
            cast: vec!["Shameik Moore".to_string(), "Jake Johnson".to_string(), "Hailee Steinfeld".to_string()],
            genres: vec!["Animation".to_string(), "Action".to_string(), "Sci-Fi".to_string()],
            runtime_minutes: 117,
            rating: 8.4,
            synopsis: "Teen Miles Morales becomes the new Spider-Man and joins other Spider-Heroes from various parallel dimensions to save the multiverse.".to_string(),
            streaming_platforms: vec!["Prime Video".to_string()],
        },
        Movie {
            id: "get-out".to_string(),
            title: "Get Out".to_string(),
            year: 2017,
            director: "Jordan Peele".to_string(),
            cast: vec!["Daniel Kaluuya".to_string(), "Allison Williams".to_string()],
            genres: vec!["Horror".to_string(), "Thriller".to_string()],
            runtime_minutes: 104,
            rating: 7.8,
            synopsis: "A young African-American visits his white girlfriend's parents for the weekend, where his simmering uneasiness about their reception reaches a boiling point.".to_string(),
            streaming_platforms: vec!["Netflix".to_string()],
        },
        Movie {
            id: "la-la-land".to_string(),
            title: "La La Land".to_string(),
            year: 2016,
            director: "Damien Chazelle".to_string(),
            cast: vec!["Ryan Gosling".to_string(), "Emma Stone".to_string()],
            genres: vec!["Drama".to_string(), "Music".to_string(), "Comedy".to_string()],
            runtime_minutes: 128,
            rating: 8.0,
            synopsis: "While navigating their careers in Los Angeles, a pianist and an actress fall in love while attempting to reconcile their aspirations.".to_string(),
            streaming_platforms: vec!["Prime Video".to_string()],
        },
    ];

    for movie in movies {
        catalog.insert(movie.id.clone(), movie);
    }

    // User 1: Alice (Sci-Fi aficionado)
    let mut alice_watchlists = HashMap::new();
    alice_watchlists.insert(
        "Sci-Fi Essentials".to_string(),
        Watchlist {
            name: "Sci-Fi Essentials".to_string(),
            description: Some("Mind-bending cosmic and philosophical science fiction".to_string()),
            created_at: "2026-08-01T10:00:00Z".to_string(),
            items: vec![
                WatchlistItem {
                    movie_id: "interstellar".to_string(),
                    added_at: "2026-08-01T10:05:00Z".to_string(),
                    notes: Some("Rewatch in 4K before weekend".to_string()),
                },
                WatchlistItem {
                    movie_id: "blade-runner-2049".to_string(),
                    added_at: "2026-08-02T14:20:00Z".to_string(),
                    notes: None,
                },
            ],
        },
    );

    let mut alice_ratings = HashMap::new();
    alice_ratings.insert(
        "2001-a-space-odyssey".to_string(),
        MovieRating {
            movie_id: "2001-a-space-odyssey".to_string(),
            rating: 9.5,
            review: Some("A monumental milestone in cinematic science fiction.".to_string()),
            watched_at: "2026-08-01T20:00:00Z".to_string(),
        },
    );
    alice_ratings.insert(
        "arrival".to_string(),
        MovieRating {
            movie_id: "arrival".to_string(),
            rating: 9.0,
            review: Some("Emotionally profound linguistic puzzle.".to_string()),
            watched_at: "2026-08-05T21:30:00Z".to_string(),
        },
    );

    let alice = UserProfile {
        user_id: "alice".to_string(),
        display_name: "Alice (Sci-Fi Buff)".to_string(),
        watchlists: alice_watchlists,
        ratings: alice_ratings,
        streaming_subscriptions: vec!["Netflix".to_string(), "Criterion Channel".to_string()],
    };

    // User 2: Bob (Thriller and Crime enthusiast)
    let mut bob_watchlists = HashMap::new();
    bob_watchlists.insert(
        "Classic Noir & Thrillers".to_string(),
        Watchlist {
            name: "Classic Noir & Thrillers".to_string(),
            description: Some("Tense, gritty mysteries and thrillers".to_string()),
            created_at: "2026-08-03T11:00:00Z".to_string(),
            items: vec![
                WatchlistItem {
                    movie_id: "parasite".to_string(),
                    added_at: "2026-08-03T11:15:00Z".to_string(),
                    notes: Some("Discuss with book club".to_string()),
                },
                WatchlistItem {
                    movie_id: "se7en".to_string(),
                    added_at: "2026-08-04T09:00:00Z".to_string(),
                    notes: None,
                },
            ],
        },
    );

    let mut bob_ratings = HashMap::new();
    bob_ratings.insert(
        "the-dark-knight".to_string(),
        MovieRating {
            movie_id: "the-dark-knight".to_string(),
            rating: 9.8,
            review: Some("Masterclass in modern suspense and character dynamics.".to_string()),
            watched_at: "2026-07-20T19:00:00Z".to_string(),
        },
    );
    bob_ratings.insert(
        "spirited-away".to_string(),
        MovieRating {
            movie_id: "spirited-away".to_string(),
            rating: 9.2,
            review: Some("Enchanting visual storytelling.".to_string()),
            watched_at: "2026-07-25T18:00:00Z".to_string(),
        },
    );

    let bob = UserProfile {
        user_id: "bob".to_string(),
        display_name: "Bob (Cinema Connoisseur)".to_string(),
        watchlists: bob_watchlists,
        ratings: bob_ratings,
        streaming_subscriptions: vec!["Max".to_string(), "Prime Video".to_string()],
    };

    let mut users = HashMap::new();
    users.insert("alice".to_string(), alice);
    users.insert("bob".to_string(), bob);

    let mut auth_tokens = HashMap::new();
    auth_tokens.insert("token_alice_secret".to_string(), "alice".to_string());
    auth_tokens.insert("token_bob_secret".to_string(), "bob".to_string());

    MovieDb {
        catalog,
        users,
        auth_tokens,
    }
}
