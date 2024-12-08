/**
 * This file was auto-generated by openapi-typescript.
 * Do not make direct changes to the file.
 */

export interface paths {
    "/authed_tokens/{authed_token_id}/": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_authed_token"];
        put?: never;
        post?: never;
        delete: operations["delete_authed_token"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/boards/": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_boards"];
        put?: never;
        post: operations["create_board"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/boards/{board_key}/": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_board"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/boards/{board_key}/admin-dat-archives/{thread_number}/": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_admin_dat_archived_thread"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/boards/{board_key}/archives/": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_archived_threads"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/boards/{board_key}/archives/{thread_id}/": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_archived_thread"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/boards/{board_key}/archives/{thread_id}/responses/": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_archived_responses"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/boards/{board_key}/dat-archives/{thread_number}/": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_dat_archived_thread"];
        put?: never;
        post?: never;
        delete: operations["delete_archived_thread"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/boards/{board_key}/dat-archives/{thread_number}/responses/": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch: operations["update_archived_res"];
        trace?: never;
    };
    "/boards/{board_key}/dat-archives/{thread_number}/responses/{res_order}/": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete: operations["delete_archived_res"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/boards/{board_key}/threads/": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_threads"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/boards/{board_key}/threads/{thread_id}/": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_thread"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/boards/{board_key}/threads/{thread_id}/responses/": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_responses"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/boards/{board_key}/threads/{thread_id}/responses/{res_id}/": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch: operations["update_response"];
        trace?: never;
    };
    "/caps/": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_caps"];
        put?: never;
        post: operations["create_cap"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/caps/{cap_id}/": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete: operations["delete_cap"];
        options?: never;
        head?: never;
        patch: operations["update_cap"];
        trace?: never;
    };
    "/ng_words/": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["get_ng_words"];
        put?: never;
        post: operations["create_ng_word"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/ng_words/{ng_word_id}/": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        delete: operations["delete_ng_word"];
        options?: never;
        head?: never;
        patch: operations["update_ng_word"];
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        ArchivedAdminRes: {
            authed_token_id: string;
            author_id?: string | null;
            body: string;
            date: string;
            ip_addr: string;
            mail: string;
            name: string;
        };
        ArchivedAdminThread: {
            responses: components["schemas"]["ArchivedAdminRes"][];
            title: string;
        };
        ArchivedRes: {
            author_id?: string | null;
            body: string;
            date: string;
            is_abone: boolean;
            mail: string;
            name: string;
            /** Format: int64 */
            order: number;
        };
        ArchivedResUpdate: {
            author_name: string;
            body: string;
            email: string;
            /** Format: int64 */
            res_order: number;
        };
        ArchivedThread: {
            responses: components["schemas"]["ArchivedRes"][];
            title: string;
        };
        AuthedToken: {
            /** Format: date-time */
            authed_at?: string | null;
            authed_ua?: string | null;
            /** Format: date-time */
            created_at: string;
            /** Format: uuid */
            id: string;
            /** Format: date-time */
            last_wrote_at?: string | null;
            origin_ip: string;
            reduced_origin_ip: string;
            token: string;
            validity: boolean;
            writing_ua: string;
        };
        Board: {
            board_key: string;
            default_name: string;
            /** Format: uuid */
            id: string;
            name: string;
            /** Format: int64 */
            thread_count: number;
        };
        Cap: {
            board_ids: string[];
            /** Format: date-time */
            created_at: string;
            description: string;
            /** Format: uuid */
            id: string;
            name: string;
            /** Format: date-time */
            updated_at: string;
        };
        ClientInfo: {
            /** Format: int32 */
            asn_num: number;
            ip_addr: string;
            tinker?: components["schemas"]["Tinker"] | null;
            user_agent: string;
        };
        CreateBoardInput: {
            base_response_creation_span_sec?: number | null;
            base_thread_creation_span_sec?: number | null;
            board_key: string;
            default_name: string;
            local_rule: string;
            max_author_name_byte_length?: number | null;
            max_email_byte_length?: number | null;
            max_response_body_byte_length?: number | null;
            max_response_body_lines?: number | null;
            max_thread_name_byte_length?: number | null;
            name: string;
            threads_archive_cron?: string | null;
            threads_archive_trigger_thread_count?: number | null;
        };
        CreationCapInput: {
            description: string;
            name: string;
            password: string;
        };
        CreationNgWordInput: {
            name: string;
            word: string;
        };
        NgWord: {
            board_ids: string[];
            /** Format: date-time */
            created_at: string;
            /** Format: uuid */
            id: string;
            name: string;
            /** Format: date-time */
            updated_at: string;
            word: string;
        };
        Res: {
            /** Format: uuid */
            authed_token_id: string;
            author_id: string;
            author_name?: string | null;
            /** Format: uuid */
            board_id: string;
            body: string;
            client_info: components["schemas"]["ClientInfo"];
            /** Format: date-time */
            created_at: string;
            /** Format: uuid */
            id: string;
            ip_addr: string;
            is_abone: boolean;
            mail?: string | null;
            /** Format: int32 */
            res_order: number;
            /** Format: uuid */
            thread_id: string;
        };
        Thread: {
            active: boolean;
            archived: boolean;
            /** Format: uuid */
            authed_token_id: string;
            /** Format: uuid */
            board_id: string;
            /** Format: uuid */
            id: string;
            /** Format: date-time */
            last_modified: string;
            metadent: string;
            no_pool: boolean;
            /** Format: int32 */
            response_count: number;
            /** Format: date-time */
            sage_last_modified: string;
            /** Format: int64 */
            thread_number: number;
            title: string;
        };
        Tinker: {
            authed_token: string;
            /** Format: int32 */
            created_thread_count: number;
            /** Format: int64 */
            last_created_thread_at?: number | null;
            /** Format: int64 */
            last_level_up_at: number;
            /** Format: int64 */
            last_wrote_at: number;
            /** Format: int32 */
            level: number;
            /** Format: int32 */
            wrote_count: number;
        };
        UpdateCapInput: {
            board_ids?: string[] | null;
            description?: string | null;
            name?: string | null;
            password?: string | null;
        };
        UpdateNgWordInput: {
            board_ids?: string[] | null;
            name?: string | null;
            word?: string | null;
        };
        UpdateResInput: {
            author_name?: string | null;
            body?: string | null;
            is_abone?: boolean | null;
            mail?: string | null;
        };
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    get_authed_token: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Authed token ID */
                authed_token_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Get authed token successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AuthedToken"];
                };
            };
        };
    };
    delete_authed_token: {
        parameters: {
            query: {
                using_origin_ip: boolean;
            };
            header?: never;
            path: {
                /** @description Authed token ID */
                authed_token_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Delete authed token successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_boards: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List boards successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Board"][];
                };
            };
        };
    };
    create_board: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateBoardInput"];
            };
        };
        responses: {
            /** @description Create board successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["CreateBoardInput"];
                };
            };
        };
    };
    get_board: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Board ID */
                board_key: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Get board successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Board"];
                };
            };
            /** @description Board not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_admin_dat_archived_thread: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Board ID */
                board_key: string;
                /** @description Thread ID */
                thread_number: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Get archived thread successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ArchivedAdminThread"];
                };
            };
        };
    };
    get_archived_threads: {
        parameters: {
            query?: {
                keyword?: string | null;
                start?: number | null;
                end?: number | null;
                page?: number | null;
                limit?: number | null;
            };
            header?: never;
            path: {
                /** @description Board ID */
                board_key: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List threads successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Thread"][];
                };
            };
        };
    };
    get_archived_thread: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Board ID */
                board_key: string;
                /** @description Thread ID */
                thread_id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Get thread successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Thread"];
                };
            };
            /** @description Thread not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_archived_responses: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Thread ID */
                thread_id: number;
                board_key: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List responses successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Res"][];
                };
            };
            /** @description Thread not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_dat_archived_thread: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Board ID */
                board_key: string;
                /** @description Thread ID */
                thread_number: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Get archived thread successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ArchivedThread"];
                };
            };
        };
    };
    delete_archived_thread: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Board ID */
                board_key: string;
                /** @description Thread ID */
                thread_number: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Delete thread successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    update_archived_res: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Board ID */
                board_key: string;
                /** @description Thread ID */
                thread_number: number;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ArchivedResUpdate"][];
            };
        };
        responses: {
            /** @description Update archived response successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
        };
    };
    delete_archived_res: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Board ID */
                board_key: string;
                /** @description Thread ID */
                thread_number: number;
                /** @description Response order */
                res_order: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Delete response successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_threads: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                board_key: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List threads successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Thread"][];
                };
            };
        };
    };
    get_thread: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Board ID */
                board_key: string;
                /** @description Thread ID */
                thread_id: number;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Get thread successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Thread"];
                };
            };
            /** @description Thread not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    get_responses: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Thread ID */
                thread_id: number;
                board_key: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List responses successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Res"][];
                };
            };
            /** @description Thread not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    update_response: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Board ID */
                board_key: string;
                /** @description Thread ID */
                thread_id: number;
                /** @description Response ID */
                res_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateResInput"];
            };
        };
        responses: {
            /** @description Update response successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Res"];
                };
            };
        };
    };
    get_caps: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List cap words successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Cap"][];
                };
            };
        };
    };
    create_cap: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreationCapInput"];
            };
        };
        responses: {
            /** @description Create cap successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Cap"];
                };
            };
        };
    };
    delete_cap: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Cap ID */
                cap_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Delete Cap successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    update_cap: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Cap ID */
                cap_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateCapInput"];
            };
        };
        responses: {
            /** @description Update cap word successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Cap"];
                };
            };
        };
    };
    get_ng_words: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List ng words successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["NgWord"][];
                };
            };
        };
    };
    create_ng_word: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreationNgWordInput"];
            };
        };
        responses: {
            /** @description Create ng word successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["NgWord"];
                };
            };
        };
    };
    delete_ng_word: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description NG word ID */
                ng_word_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Delete ng word successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
        };
    };
    update_ng_word: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description NG word ID */
                ng_word_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateNgWordInput"];
            };
        };
        responses: {
            /** @description Update ng word successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["NgWord"];
                };
            };
        };
    };
}
